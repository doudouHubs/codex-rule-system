use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::ptr::null_mut;
use std::sync::mpsc::{Sender, channel};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CLIP_DEFAULT_PRECIS, CreateFontW, CreatePen, CreateSolidBrush, DEFAULT_CHARSET,
    DEFAULT_QUALITY, DT_CENTER, DT_SINGLELINE, DT_VCENTER, DeleteObject, DrawTextW, EndPaint,
    FF_DONTCARE, FW_NORMAL, FillRect, HBRUSH, HDC, HFONT, InvalidateRect, OUT_DEFAULT_PRECIS,
    PAINTSTRUCT, PS_SOLID, RoundRect, SelectObject, SetBkColor, SetBkMode, SetTextColor,
    TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    ICC_LISTVIEW_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx, LVCF_FMT, LVCF_TEXT,
    LVCF_WIDTH, LVCFMT_LEFT, LVCOLUMNW, LVIF_STATE, LVIF_TEXT, LVIS_FOCUSED, LVIS_SELECTED,
    LVIS_STATEIMAGEMASK, LVITEMW, LVM_DELETEALLITEMS, LVM_GETNEXTITEM, LVM_INSERTCOLUMNW,
    LVM_INSERTITEMW, LVM_SETEXTENDEDLISTVIEWSTYLE, LVM_SETITEMSTATE, LVM_SETITEMTEXTW,
    LVN_ITEMCHANGED, LVNI_SELECTED, LVS_EX_CHECKBOXES, LVS_EX_FULLROWSELECT, LVS_EX_GRIDLINES,
    LVS_REPORT, LVS_SHOWSELALWAYS, NM_CLICK, NMHDR, NMLISTVIEW, WC_LISTVIEWW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{SetFocus, VK_ESCAPE, VK_RETURN};
use windows::Win32::UI::WindowsAndMessaging::{
    ACCEL, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateAcceleratorTableW,
    CreateWindowExW, DefWindowProcW, DestroyAcceleratorTable, DestroyWindow, DispatchMessageW,
    ES_AUTOHSCROLL, ES_AUTOVSCROLL, ES_MULTILINE, FVIRTKEY, GWLP_USERDATA, GetClientRect,
    GetMessageW, GetParent, GetWindowLongPtrW, HMENU, LoadCursorW, MSG, PostQuitMessage,
    RegisterClassW, SW_RESTORE, SWP_NOZORDER, SendMessageW, SetWindowLongPtrW, SetWindowPos,
    SetWindowTextW, ShowWindow, TranslateAcceleratorW, TranslateMessage, WINDOW_STYLE, WM_CLOSE,
    WM_COMMAND, WM_CREATE, WM_CTLCOLORDLG, WM_CTLCOLOREDIT, WM_CTLCOLORLISTBOX, WM_CTLCOLORSTATIC,
    WM_DESTROY, WM_ERASEBKGND, WM_LBUTTONDOWN, WM_NOTIFY, WM_PAINT, WM_SETFONT, WM_SIZE, WNDCLASSW,
    WS_BORDER, WS_CHILD, WS_OVERLAPPEDWINDOW, WS_VISIBLE, WS_VSCROLL,
};
use windows::core::PCWSTR;

const ID_SEARCH: usize = 1001;
const ID_LIST: usize = 1002;
const ID_CONFIRM: usize = 1003;
const ID_CANCEL: usize = 1004;
const ID_TITLE: usize = 1005;
const ID_CONTENT: usize = 1006;
const ID_TAGS: usize = 1007;
const ID_STATUS: usize = 1008;
const ID_SAVE_EDIT: usize = 1009;

#[derive(Clone, Debug, Deserialize)]
struct RuleInput {
    id: String,
    title: String,
    content: String,
    #[serde(default = "default_status")]
    status: String,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Clone, Debug)]
struct RuleItem {
    id: String,
    title: String,
    content: String,
    status: String,
    tags: Vec<String>,
    original_title: String,
    original_content: String,
    original_status: String,
    original_tags: Vec<String>,
    display: String,
    search_text: String,
}

#[derive(Debug)]
enum PickerAction {
    Pick(PickerResult),
    Cancel,
}

#[derive(Debug)]
struct PickerResult {
    selected_ids: Vec<String>,
    updates: Vec<PickerUpdate>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PickerUpdate {
    id: String,
    title: String,
    content: String,
    tags: Vec<String>,
    status: String,
}

#[derive(Serialize)]
struct PickerOutput {
    selected_ids: Vec<String>,
    updates: Vec<PickerUpdate>,
    cancelled: bool,
}

struct CreateParams {
    sender: Sender<PickerAction>,
    rules: Vec<RuleItem>,
    initial_query: String,
}

struct WindowState {
    sender: Sender<PickerAction>,
    rules: Vec<RuleItem>,
    visible_indices: Vec<usize>,
    checked_rule_ids: HashSet<String>,
    search: HWND,
    list: HWND,
    title_edit: HWND,
    content_edit: HWND,
    tags_edit: HWND,
    status_switch: HWND,
    save_button: HWND,
    confirm_button: HWND,
    cancel_button: HWND,
    bg_brush: HBRUSH,
    input_brush: HBRUSH,
    font: HFONT,
    action_sent: bool,
    editing_rule_index: Option<usize>,
    status_value: String,
    title_label: HWND,
    hint_label: HWND,
    search_label: HWND,
    list_status_label: HWND,
    editor_heading_label: HWND,
    editor_hint_label: HWND,
    title_field_label: HWND,
    content_field_label: HWND,
    tags_field_label: HWND,
    status_field_label: HWND,
    font_title: HFONT,
}

fn main() {
    let result = run();
    match result {
        Ok(output) => {
            println!(
                "{}",
                serde_json::to_string(&output)
                    .unwrap_or_else(|_| { "{\"selected_ids\":[],\"cancelled\":true}".to_string() })
            );
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<PickerOutput, String> {
    if let Some(output) = headless_output_from_env() {
        return Ok(output);
    }

    let args = Args::parse(env::args().skip(1).collect())?;
    let rules = load_rules(&args.rules_file)?;

    match run_picker_window(rules, args.query) {
        PickerAction::Pick(result) => Ok(PickerOutput {
            selected_ids: result.selected_ids,
            updates: result.updates,
            cancelled: false,
        }),
        PickerAction::Cancel => Ok(PickerOutput {
            selected_ids: Vec::new(),
            updates: Vec::new(),
            cancelled: true,
        }),
    }
}

struct Args {
    rules_file: PathBuf,
    query: String,
}

impl Args {
    fn parse(args: Vec<String>) -> Result<Self, String> {
        let mut rules_file: Option<PathBuf> = None;
        let mut query = String::new();
        let mut index = 0usize;
        while index < args.len() {
            match args[index].as_str() {
                "--rules" => {
                    index += 1;
                    let value = args
                        .get(index)
                        .ok_or_else(|| "--rules requires a file path".to_string())?;
                    rules_file = Some(PathBuf::from(value));
                }
                "--query" => {
                    index += 1;
                    query = args
                        .get(index)
                        .ok_or_else(|| "--query requires a value".to_string())?
                        .to_string();
                }
                "--help" | "-h" => {
                    return Err(
                        "Usage: rule-picker-win --rules <rules.json> [--query <text>]".to_string(),
                    );
                }
                value => return Err(format!("Unknown argument: {value}")),
            }
            index += 1;
        }

        let rules_file = rules_file.ok_or_else(|| "--rules is required".to_string())?;
        Ok(Self { rules_file, query })
    }
}

fn headless_output_from_env() -> Option<PickerOutput> {
    if truthy_env("RULE_PICKER_HEADLESS_CANCEL") {
        return Some(PickerOutput {
            selected_ids: Vec::new(),
            updates: Vec::new(),
            cancelled: true,
        });
    }
    let ids = env::var("RULE_PICKER_HEADLESS_IDS").ok()?;
    let updates = env::var("RULE_PICKER_HEADLESS_UPDATES")
        .ok()
        .and_then(|raw| serde_json::from_str::<Vec<PickerUpdate>>(&raw).ok())
        .unwrap_or_default();
    Some(PickerOutput {
        selected_ids: split_ids(&ids),
        updates,
        cancelled: false,
    })
}

fn truthy_env(name: &str) -> bool {
    env::var(name)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes"
            )
        })
        .unwrap_or(false)
}

fn split_ids(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn load_rules(path: &PathBuf) -> Result<Vec<RuleItem>, String> {
    let content = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let raw_rules: Vec<RuleInput> =
        serde_json::from_str(&content).map_err(|err| err.to_string())?;
    let rules = raw_rules
        .into_iter()
        .filter_map(|rule| {
            let id = rule.id.trim().to_string();
            let title = rule.title.trim().to_string();
            let content = rule.content.trim().to_string();
            let status = normalize_status(&rule.status);
            if id.is_empty() || title.is_empty() || content.is_empty() {
                return None;
            }

            let tags = normalize_tags(rule.tags);
            let mut item = RuleItem {
                id,
                title,
                content,
                status,
                tags,
                original_title: String::new(),
                original_content: String::new(),
                original_status: String::new(),
                original_tags: Vec::new(),
                display: String::new(),
                search_text: String::new(),
            };
            item.original_title = item.title.clone();
            item.original_content = item.content.clone();
            item.original_status = item.status.clone();
            item.original_tags = item.tags.clone();
            refresh_rule_text(&mut item);
            Some(item)
        })
        .collect::<Vec<_>>();
    Ok(rules)
}

fn default_status() -> String {
    "active".to_string()
}

fn normalize_status(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "deprecated" => "deprecated".to_string(),
        _ => "active".to_string(),
    }
}

fn normalize_tags(raw: Vec<String>) -> Vec<String> {
    raw.into_iter()
        .flat_map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|segment| !segment.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .fold(Vec::new(), |mut tags, tag| {
            if !tags.iter().any(|existing| existing == &tag) {
                tags.push(tag);
            }
            tags
        })
}

fn refresh_rule_text(rule: &mut RuleItem) {
    let tags = rule.tags.join(",");
    rule.display = format!(
        "{} [{}] [{}] {}: {}",
        rule.id, rule.status, tags, rule.title, rule.content
    );
    rule.search_text = format!(
        "{} {} {} {} {}",
        rule.id.to_ascii_lowercase(),
        rule.status.to_lowercase(),
        rule.title.to_lowercase(),
        rule.content.to_lowercase(),
        tags.to_lowercase()
    );
}

fn run_picker_window(rules: Vec<RuleItem>, initial_query: String) -> PickerAction {
    let (sender, receiver) = channel();

    unsafe {
        let common_controls = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_LISTVIEW_CLASSES,
        };
        let _ = InitCommonControlsEx(&common_controls);

        let hinstance = GetModuleHandleW(None).unwrap();
        let class_name = to_wstring("RulePickerWindow");
        let wnd_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance.into(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            hCursor: LoadCursorW(None, windows::Win32::UI::WindowsAndMessaging::IDC_ARROW).unwrap(),
            ..Default::default()
        };
        let _ = RegisterClassW(&wnd_class);

        let switch_class_name = to_wstring("RuleStatusSwitch");
        let switch_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(status_switch_proc),
            hInstance: hinstance.into(),
            lpszClassName: PCWSTR(switch_class_name.as_ptr()),
            hCursor: LoadCursorW(None, windows::Win32::UI::WindowsAndMessaging::IDC_HAND)
                .unwrap_or_default(),
            ..Default::default()
        };
        let _ = RegisterClassW(&switch_class);

        let params = Box::new(CreateParams {
            sender,
            rules,
            initial_query,
        });
        let hwnd = CreateWindowExW(
            Default::default(),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(to_wstring("Rule Check").as_ptr()),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1120,
            720,
            None,
            None,
            hinstance,
            Some(Box::into_raw(params) as *const _),
        )
        .unwrap_or(HWND(null_mut()));

        if hwnd.0.is_null() {
            return PickerAction::Cancel;
        }

        let _ = ShowWindow(hwnd, SW_RESTORE);
        let accelerators = [
            ACCEL {
                fVirt: FVIRTKEY,
                key: VK_RETURN.0 as u16,
                cmd: ID_CONFIRM as u16,
            },
            ACCEL {
                fVirt: FVIRTKEY,
                key: VK_ESCAPE.0 as u16,
                cmd: ID_CANCEL as u16,
            },
        ];
        let accel_table = CreateAcceleratorTableW(&accelerators).unwrap_or_default();

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND(null_mut()), 0, 0).into() {
            if TranslateAcceleratorW(hwnd, accel_table, &msg) != 0 {
                continue;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let _ = DestroyAcceleratorTable(accel_table);
    }

    receiver.recv().unwrap_or(PickerAction::Cancel)
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let createstruct = unsafe { &*(lparam.0 as *const CREATESTRUCTW) };
            let params = unsafe { Box::from_raw(createstruct.lpCreateParams as *mut CreateParams) };

            let bg_brush = unsafe { CreateSolidBrush(rgb(244, 246, 248)) };
            let input_brush = unsafe { CreateSolidBrush(rgb(255, 255, 255)) };
            let font = unsafe {
                CreateFontW(
                    -16,
                    0,
                    0,
                    0,
                    FW_NORMAL.0 as i32,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET.0 as u32,
                    OUT_DEFAULT_PRECIS.0 as u32,
                    CLIP_DEFAULT_PRECIS.0 as u32,
                    DEFAULT_QUALITY.0 as u32,
                    FF_DONTCARE.0 as u32,
                    PCWSTR(to_wstring("Microsoft YaHei UI").as_ptr()),
                )
            };
            let font_title = unsafe {
                CreateFontW(
                    -24,
                    0,
                    0,
                    0,
                    700,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET.0 as u32,
                    OUT_DEFAULT_PRECIS.0 as u32,
                    CLIP_DEFAULT_PRECIS.0 as u32,
                    DEFAULT_QUALITY.0 as u32,
                    FF_DONTCARE.0 as u32,
                    PCWSTR(to_wstring("Microsoft YaHei UI").as_ptr()),
                )
            };

            let title_label = create_label(hwnd, "Rule Check", 18, 18, 220, 32);
            let hint_label = create_label(
                hwnd,
                "复选框 = 最终 pick；高亮行 = 右侧编辑对象。搜索只过滤可见行，不会清空已勾选规则。",
                252,
                24,
                820,
                24,
            );
            let search_label = create_label(hwnd, "搜索项目规则", 18, 66, 220, 22);
            let search = create_edit(hwnd, &params.initial_query, 18, 90, 640, 30, ID_SEARCH);
            let list_status_label = create_label(hwnd, "", 18, 128, 640, 22);
            let list = create_list(hwnd, 18, 154, 640, 438, ID_LIST);
            let editor_heading_label = create_label(hwnd, "编辑当前规则", 690, 66, 360, 28);
            let editor_hint_label = create_label(
                hwnd,
                "只保存右侧当前高亮行；未勾选不会被写入当前 session。",
                690,
                96,
                360,
                22,
            );
            let title_field_label = create_label(hwnd, "标题", 690, 132, 360, 22);
            let title_edit = create_edit(hwnd, "", 690, 156, 372, 30, ID_TITLE);
            let content_field_label = create_label(hwnd, "内容", 690, 202, 360, 22);
            let content_edit = create_multiline_edit(hwnd, "", 690, 226, 372, 228, ID_CONTENT);
            let tags_field_label = create_label(hwnd, "标签（逗号分隔）", 690, 470, 360, 22);
            let tags_edit = create_edit(hwnd, "", 690, 494, 372, 30, ID_TAGS);
            let status_field_label = create_label(hwnd, "状态", 690, 540, 360, 22);
            let status_switch = create_status_switch(hwnd, 690, 564, 240, 34, ID_STATUS);
            let save_button = create_button(hwnd, "保存编辑", 690, 614, 150, 36, ID_SAVE_EDIT);
            let cancel_button = create_button(hwnd, "取消 Esc", 756, 614, 140, 36, ID_CANCEL);
            let confirm_button =
                create_button(hwnd, "保存并选取 Enter", 916, 614, 156, 36, ID_CONFIRM);

            set_font(title_label, font_title);
            set_font(hint_label, font);
            set_font(search_label, font);
            set_font(search, font);
            set_font(list_status_label, font);
            set_font(list, font);
            set_font(editor_heading_label, font_title);
            set_font(editor_hint_label, font);
            set_font(title_field_label, font);
            set_font(title_edit, font);
            set_font(content_field_label, font);
            set_font(content_edit, font);
            set_font(tags_field_label, font);
            set_font(tags_edit, font);
            set_font(status_field_label, font);
            set_font(save_button, font);
            set_font(cancel_button, font);
            set_font(confirm_button, font);

            let mut state = Box::new(WindowState {
                sender: params.sender,
                rules: params.rules,
                visible_indices: Vec::new(),
                checked_rule_ids: HashSet::new(),
                search,
                list,
                title_edit,
                content_edit,
                tags_edit,
                status_switch,
                save_button,
                confirm_button,
                cancel_button,
                bg_brush,
                input_brush,
                font,
                font_title,
                action_sent: false,
                editing_rule_index: None,
                status_value: "active".to_string(),
                title_label,
                hint_label,
                search_label,
                list_status_label,
                editor_heading_label,
                editor_hint_label,
                title_field_label,
                content_field_label,
                tags_field_label,
                status_field_label,
            });
            refresh_visible_rules(&mut state);
            select_first_visible_rule(&mut state);
            load_editor_from_current_selection(&mut state);
            let state_ptr = Box::into_raw(state);
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
                apply_layout(hwnd, &*state_ptr);
                let _ = SetFocus(search);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = loword(wparam.0 as u32) as usize;
            let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState };
            if ptr.is_null() {
                return LRESULT(0);
            }
            let state = unsafe { &mut *ptr };
            match id {
                ID_SEARCH => {
                    save_editor_to_current_rule(state);
                    refresh_visible_rules(state);
                    select_first_visible_rule(state);
                    load_editor_from_current_selection(state);
                }
                ID_SAVE_EDIT => {
                    save_editor_to_current_rule(state);
                    refresh_visible_rules(state);
                    restore_editor_selection(state);
                    load_editor_from_current_selection(state);
                }
                ID_CONFIRM => finish_with_selection(hwnd, state),
                ID_CANCEL => finish_cancelled(hwnd, state),
                _ => {}
            }
            LRESULT(0)
        }
        WM_NOTIFY => {
            let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState };
            if ptr.is_null() {
                return LRESULT(0);
            }
            let state = unsafe { &mut *ptr };
            handle_list_notify(state, lparam);
            LRESULT(0)
        }
        WM_SIZE => {
            let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState };
            if !ptr.is_null() {
                let state = unsafe { &*ptr };
                apply_layout(hwnd, state);
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState };
            if !ptr.is_null() {
                let state = unsafe { &mut *ptr };
                finish_cancelled(hwnd, state);
            } else {
                unsafe {
                    let _ = DestroyWindow(hwnd);
                }
            }
            LRESULT(0)
        }
        WM_ERASEBKGND => {
            let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState };
            if ptr.is_null() {
                return LRESULT(0);
            }
            let state = unsafe { &*ptr };
            let hdc = HDC(wparam.0 as *mut core::ffi::c_void);
            let mut rect = RECT::default();
            unsafe {
                let _ = GetClientRect(hwnd, &mut rect);
                let _ = FillRect(hdc, &rect, state.bg_brush);
            }
            LRESULT(1)
        }
        WM_CTLCOLORDLG | WM_CTLCOLORSTATIC => {
            let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState };
            if ptr.is_null() {
                return LRESULT(0);
            }
            let state = unsafe { &*ptr };
            let hdc = HDC(wparam.0 as *mut core::ffi::c_void);
            let control = HWND(lparam.0 as *mut core::ffi::c_void);
            let text_color =
                if control == state.title_label || control == state.editor_heading_label {
                    rgb(18, 34, 58)
                } else if control == state.hint_label
                    || control == state.editor_hint_label
                    || control == state.list_status_label
                {
                    rgb(82, 91, 105)
                } else {
                    rgb(48, 52, 58)
                };
            unsafe {
                SetBkMode(hdc, TRANSPARENT);
                SetTextColor(hdc, text_color);
                SetBkColor(hdc, rgb(244, 246, 248));
            }
            LRESULT(state.bg_brush.0 as isize)
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => {
            let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState };
            if ptr.is_null() {
                return LRESULT(0);
            }
            let state = unsafe { &*ptr };
            let hdc = HDC(wparam.0 as *mut core::ffi::c_void);
            unsafe {
                SetBkMode(hdc, TRANSPARENT);
                SetTextColor(hdc, rgb(28, 32, 36));
                SetBkColor(hdc, rgb(255, 255, 255));
            }
            LRESULT(state.input_brush.0 as isize)
        }
        WM_DESTROY => {
            let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState };
            let _ = unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };
            if !ptr.is_null() {
                let state = unsafe { Box::from_raw(ptr) };
                if !state.action_sent {
                    let _ = state.sender.send(PickerAction::Cancel);
                }
                unsafe {
                    let _ = DeleteObject(state.bg_brush);
                    let _ = DeleteObject(state.input_brush);
                    let _ = DeleteObject(state.font);
                    let _ = DeleteObject(state.font_title);
                }
            }
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

unsafe extern "system" fn status_switch_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let hdc = unsafe { BeginPaint(hwnd, &mut paint) };
            draw_status_switch(hwnd, hdc);
            unsafe {
                let _ = EndPaint(hwnd, &paint);
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let parent = unsafe { GetParent(hwnd).unwrap_or(HWND(null_mut())) };
            let ptr = unsafe { GetWindowLongPtrW(parent, GWLP_USERDATA) as *mut WindowState };
            if ptr.is_null() {
                return LRESULT(0);
            }

            let mut rect = RECT::default();
            unsafe {
                let _ = GetClientRect(hwnd, &mut rect);
            }
            let width = (rect.right - rect.left).max(1);
            let x = (lparam.0 & 0xffff) as i16 as i32;
            let next_status = if x < width / 2 {
                "active"
            } else {
                "deprecated"
            };

            let state = unsafe { &mut *ptr };
            if state.status_value != next_status {
                state.status_value = next_status.to_string();
                invalidate_status_switch(state);
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn draw_status_switch(hwnd: HWND, hdc: HDC) {
    let status = current_switch_status(hwnd);
    let active_selected = status == "active";
    let mut rect = RECT::default();
    unsafe {
        let _ = GetClientRect(hwnd, &mut rect);
    }
    let width = (rect.right - rect.left).max(1);
    let height = (rect.bottom - rect.top).max(1);
    let mid = width / 2;

    let track_brush = unsafe { CreateSolidBrush(rgb(232, 236, 240)) };
    let track_pen = unsafe { CreatePen(PS_SOLID, 1, rgb(177, 186, 198)) };
    let old_brush = unsafe { SelectObject(hdc, track_brush) };
    let old_pen = unsafe { SelectObject(hdc, track_pen) };
    unsafe {
        let _ = RoundRect(hdc, 0, 0, width, height, height, height);
    }

    let selected_color = if active_selected {
        rgb(35, 134, 84)
    } else {
        rgb(118, 124, 134)
    };
    let selected_brush = unsafe { CreateSolidBrush(selected_color) };
    let selected_pen = unsafe { CreatePen(PS_SOLID, 1, selected_color) };
    unsafe {
        let _ = SelectObject(hdc, selected_brush);
        let _ = SelectObject(hdc, selected_pen);
        if active_selected {
            // 选中块略微跨过中线，避免胶囊中间出现一条突兀空隙。
            let _ = RoundRect(hdc, 2, 2, mid + 14, height - 2, height - 4, height - 4);
        } else {
            let _ = RoundRect(
                hdc,
                mid - 14,
                2,
                width - 2,
                height - 2,
                height - 4,
                height - 4,
            );
        }
    }

    unsafe {
        let _ = SelectObject(hdc, old_brush);
        let _ = SelectObject(hdc, old_pen);
        let _ = DeleteObject(track_brush);
        let _ = DeleteObject(track_pen);
        let _ = DeleteObject(selected_brush);
        let _ = DeleteObject(selected_pen);
        SetBkMode(hdc, TRANSPARENT);
    }

    let mut active_rect = RECT {
        left: 0,
        top: 0,
        right: mid,
        bottom: height,
    };
    let mut deprecated_rect = RECT {
        left: mid,
        top: 0,
        right: width,
        bottom: height,
    };
    draw_switch_text(hdc, "active", &mut active_rect, active_selected);
    draw_switch_text(hdc, "deprecated", &mut deprecated_rect, !active_selected);
}

fn draw_switch_text(hdc: HDC, text: &str, rect: &mut RECT, selected: bool) {
    let mut text = to_wstring(text);
    // DrawTextW 的 windows-rs 包装按 slice 长度绘制；去掉 C 字符串终止符，
    // 否则部分字体会把结尾 NUL 显示成异常占位字符。
    let _ = text.pop();
    if selected {
        unsafe {
            SetTextColor(hdc, rgb(255, 255, 255));
        }
    } else {
        unsafe {
            SetTextColor(hdc, rgb(57, 64, 75));
        }
    }
    let format = DT_CENTER | DT_VCENTER | DT_SINGLELINE;
    unsafe {
        let _ = DrawTextW(hdc, &mut text, rect, format);
    }
}

fn current_switch_status(hwnd: HWND) -> String {
    let parent = unsafe { GetParent(hwnd).unwrap_or(HWND(null_mut())) };
    let ptr = unsafe { GetWindowLongPtrW(parent, GWLP_USERDATA) as *const WindowState };
    if ptr.is_null() {
        return "active".to_string();
    }
    let state = unsafe { &*ptr };
    normalize_status(&state.status_value)
}

fn refresh_visible_rules(state: &mut WindowState) {
    let query = read_window_text(state.search);
    let terms = query
        .to_lowercase()
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    state.visible_indices.clear();
    unsafe {
        let _ = SendMessageW(state.list, LVM_DELETEALLITEMS, WPARAM(0), LPARAM(0));
    }

    for index in 0..state.rules.len() {
        let rule = state.rules[index].clone();
        if !terms.iter().all(|term| rule.search_text.contains(term)) {
            continue;
        }
        state.visible_indices.push(index);
        let row_index = state.visible_indices.len() - 1;
        insert_rule_row(state, row_index, &rule);
    }

    if state.visible_indices.is_empty() {
        let message = if state.rules.is_empty() {
            "当前无项目规则。请先用 rule-add --scope project 新增。"
        } else {
            "没有匹配的项目规则。请调整搜索关键词。"
        };
        insert_empty_row(state.list, message);
    }
    update_status_label(state);
}

fn insert_rule_row(state: &WindowState, row_index: usize, rule: &RuleItem) {
    let checked = state.checked_rule_ids.contains(&rule.id);
    let preview = content_preview(&rule.content);
    let tags = rule.tags.join(",");
    let values = [
        "",
        rule.id.as_str(),
        rule.status.as_str(),
        tags.as_str(),
        rule.title.as_str(),
        preview.as_str(),
    ];

    insert_list_view_item(state.list, row_index, values[0]);
    for (subitem, value) in values.iter().enumerate().skip(1) {
        set_list_view_subitem(state.list, row_index, subitem, value);
    }
    set_list_view_checked(state.list, row_index, checked);
}

fn insert_empty_row(list: HWND, message: &str) {
    insert_list_view_item(list, 0, "");
    set_list_view_subitem(list, 0, 4, message);
    set_list_view_subitem(list, 0, 5, message);
    clear_list_view_checkbox(list, 0);
}

fn insert_list_view_item(list: HWND, row_index: usize, text: &str) {
    let mut text = to_wstring(text);
    let mut item = LVITEMW {
        mask: LVIF_TEXT,
        iItem: row_index as i32,
        iSubItem: 0,
        pszText: windows::core::PWSTR(text.as_mut_ptr()),
        ..Default::default()
    };
    unsafe {
        let _ = SendMessageW(
            list,
            LVM_INSERTITEMW,
            WPARAM(0),
            LPARAM((&mut item as *mut LVITEMW) as isize),
        );
    }
}

fn set_list_view_subitem(list: HWND, row_index: usize, subitem: usize, text: &str) {
    let mut text = to_wstring(text);
    let mut item = LVITEMW {
        iSubItem: subitem as i32,
        pszText: windows::core::PWSTR(text.as_mut_ptr()),
        ..Default::default()
    };
    unsafe {
        let _ = SendMessageW(
            list,
            LVM_SETITEMTEXTW,
            WPARAM(row_index),
            LPARAM((&mut item as *mut LVITEMW) as isize),
        );
    }
}

fn set_list_view_checked(list: HWND, row_index: usize, checked: bool) {
    let state_image = if checked { 2u32 } else { 1u32 } << 12;
    let mut item = LVITEMW {
        mask: LVIF_STATE,
        state: windows::Win32::UI::Controls::LIST_VIEW_ITEM_STATE_FLAGS(state_image),
        stateMask: LVIS_STATEIMAGEMASK,
        ..Default::default()
    };
    unsafe {
        let _ = SendMessageW(
            list,
            LVM_SETITEMSTATE,
            WPARAM(row_index),
            LPARAM((&mut item as *mut LVITEMW) as isize),
        );
    }
}

fn clear_list_view_checkbox(list: HWND, row_index: usize) {
    let mut item = LVITEMW {
        mask: LVIF_STATE,
        state: windows::Win32::UI::Controls::LIST_VIEW_ITEM_STATE_FLAGS(0),
        stateMask: LVIS_STATEIMAGEMASK,
        ..Default::default()
    };
    unsafe {
        let _ = SendMessageW(
            list,
            LVM_SETITEMSTATE,
            WPARAM(row_index),
            LPARAM((&mut item as *mut LVITEMW) as isize),
        );
    }
}

fn list_view_checked_state(raw_state: u32) -> bool {
    ((raw_state & LVIS_STATEIMAGEMASK.0) >> 12) == 2
}

fn content_preview(content: &str) -> String {
    const MAX_CHARS: usize = 80;
    let normalized = content.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut preview = normalized.chars().take(MAX_CHARS).collect::<String>();
    if normalized.chars().count() > MAX_CHARS {
        preview.push('…');
    }
    preview
}

fn handle_list_notify(state: &mut WindowState, lparam: LPARAM) {
    if lparam.0 == 0 {
        return;
    }
    let header = unsafe { &*(lparam.0 as *const NMHDR) };
    if header.idFrom != ID_LIST {
        return;
    }
    match header.code {
        LVN_ITEMCHANGED => handle_list_item_changed(state, lparam),
        NM_CLICK => {
            save_editor_to_current_rule(state);
            load_editor_from_current_selection(state);
        }
        _ => {}
    }
}

fn handle_list_item_changed(state: &mut WindowState, lparam: LPARAM) {
    let event = unsafe { &*(lparam.0 as *const NMLISTVIEW) };
    if event.iItem < 0 {
        return;
    }
    let visible_index = event.iItem as usize;
    let Some(rule_index) = state.visible_indices.get(visible_index).copied() else {
        return;
    };

    if (event.uChanged.0 & LVIF_STATE.0) != 0
        && ((event.uNewState ^ event.uOldState) & LVIS_STATEIMAGEMASK.0) != 0
    {
        let rule_id = state.rules[rule_index].id.clone();
        if list_view_checked_state(event.uNewState) {
            state.checked_rule_ids.insert(rule_id);
        } else {
            state.checked_rule_ids.remove(&rule_id);
        }
        update_status_label(state);
    }

    if (event.uChanged.0 & LVIF_STATE.0) != 0
        && ((event.uNewState ^ event.uOldState) & (LVIS_SELECTED.0 | LVIS_FOCUSED.0)) != 0
        && (event.uNewState & (LVIS_SELECTED.0 | LVIS_FOCUSED.0)) != 0
    {
        save_editor_to_current_rule(state);
        load_editor_from_current_selection(state);
    }
}

fn finish_with_selection(hwnd: HWND, state: &mut WindowState) {
    save_editor_to_current_rule(state);

    let selected_ids = state
        .rules
        .iter()
        .filter(|rule| state.checked_rule_ids.contains(&rule.id))
        .map(|rule| rule.id.clone())
        .collect::<Vec<_>>();

    state.action_sent = true;
    let _ = state.sender.send(PickerAction::Pick(PickerResult {
        selected_ids,
        updates: collect_updates(state),
    }));
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

fn select_first_visible_rule(state: &mut WindowState) {
    if state.visible_indices.is_empty() {
        state.editing_rule_index = None;
        return;
    }
    select_visible_row(state.list, 0);
}

fn restore_editor_selection(state: &mut WindowState) {
    let Some(rule_index) = state.editing_rule_index else {
        select_first_visible_rule(state);
        return;
    };
    let Some(visible_index) = state
        .visible_indices
        .iter()
        .position(|candidate| *candidate == rule_index)
    else {
        select_first_visible_rule(state);
        return;
    };
    select_visible_row(state.list, visible_index);
}

fn select_visible_row(list: HWND, visible_index: usize) {
    let focused_selected =
        windows::Win32::UI::Controls::LIST_VIEW_ITEM_STATE_FLAGS(LVIS_SELECTED.0 | LVIS_FOCUSED.0);
    let mut item = LVITEMW {
        mask: LVIF_STATE,
        state: focused_selected,
        stateMask: focused_selected,
        ..Default::default()
    };
    unsafe {
        let _ = SendMessageW(
            list,
            LVM_SETITEMSTATE,
            WPARAM(visible_index),
            LPARAM((&mut item as *mut LVITEMW) as isize),
        );
    }
}

fn load_editor_from_current_selection(state: &mut WindowState) {
    let visible_index = current_visible_index(state);
    let Some(visible_index) = visible_index else {
        state.editing_rule_index = None;
        set_window_text(state.title_edit, "");
        set_window_text(state.content_edit, "");
        set_window_text(state.tags_edit, "");
        state.status_value = "active".to_string();
        invalidate_status_switch(state);
        update_status_label(state);
        return;
    };
    let Some(rule_index) = state.visible_indices.get(visible_index).copied() else {
        state.editing_rule_index = None;
        update_status_label(state);
        return;
    };
    let rule = &state.rules[rule_index];
    state.editing_rule_index = Some(rule_index);
    set_window_text(state.title_edit, &rule.title);
    set_window_text(state.content_edit, &rule.content);
    set_window_text(state.tags_edit, &rule.tags.join(","));
    state.status_value = rule.status.clone();
    invalidate_status_switch(state);
    update_status_label(state);
}

fn save_editor_to_current_rule(state: &mut WindowState) {
    let Some(rule_index) = state.editing_rule_index else {
        return;
    };
    let Some(rule) = state.rules.get_mut(rule_index) else {
        return;
    };
    rule.title = read_window_text(state.title_edit).trim().to_string();
    rule.content = read_window_text(state.content_edit).trim().to_string();
    rule.tags = split_tags(&read_window_text(state.tags_edit));
    rule.status = normalize_status(&state.status_value);
    refresh_rule_text(rule);
    update_status_label(state);
}

fn update_status_label(state: &WindowState) {
    let checked_count = state.checked_rule_ids.len();
    let visible_count = state.visible_indices.len();
    let total_count = state.rules.len();
    let editing = state
        .editing_rule_index
        .and_then(|index| state.rules.get(index))
        .map(|rule| format!("当前编辑：{} · {}", rule.id, rule.title))
        .unwrap_or_else(|| "当前编辑：无".to_string());
    set_window_text(
        state.list_status_label,
        &format!(
            "已勾选 {checked_count} 条 · 可见 {visible_count} / 总计 {total_count} · {editing}"
        ),
    );
}

fn invalidate_status_switch(state: &WindowState) {
    unsafe {
        let _ = InvalidateRect(state.status_switch, None, true);
    }
}

fn current_visible_index(state: &WindowState) -> Option<usize> {
    if state.visible_indices.is_empty() {
        return None;
    }
    let raw_index = unsafe {
        SendMessageW(
            state.list,
            LVM_GETNEXTITEM,
            WPARAM(usize::MAX),
            LPARAM(LVNI_SELECTED as isize),
        )
        .0
    };
    if raw_index >= 0 {
        let index = raw_index as usize;
        if index < state.visible_indices.len() {
            return Some(index);
        }
    }
    None
}

fn collect_updates(state: &WindowState) -> Vec<PickerUpdate> {
    state
        .rules
        .iter()
        .filter(|rule| {
            rule.title != rule.original_title
                || rule.content != rule.original_content
                || rule.status != rule.original_status
                || rule.tags != rule.original_tags
        })
        .map(|rule| PickerUpdate {
            id: rule.id.clone(),
            title: rule.title.clone(),
            content: rule.content.clone(),
            tags: rule.tags.clone(),
            status: rule.status.clone(),
        })
        .collect()
}

fn finish_cancelled(hwnd: HWND, state: &mut WindowState) {
    state.action_sent = true;
    let _ = state.sender.send(PickerAction::Cancel);
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

fn create_edit(hwnd: HWND, text: &str, x: i32, y: i32, width: i32, height: i32, id: usize) -> HWND {
    unsafe {
        CreateWindowExW(
            Default::default(),
            PCWSTR(to_wstring("EDIT").as_ptr()),
            PCWSTR(to_wstring(text).as_ptr()),
            WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | WS_BORDER.0 | ES_AUTOHSCROLL as u32),
            x,
            y,
            width,
            height,
            hwnd,
            HMENU(id as *mut core::ffi::c_void),
            None,
            None,
        )
        .unwrap_or(HWND(null_mut()))
    }
}

fn create_multiline_edit(
    hwnd: HWND,
    text: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    id: usize,
) -> HWND {
    unsafe {
        CreateWindowExW(
            Default::default(),
            PCWSTR(to_wstring("EDIT").as_ptr()),
            PCWSTR(to_wstring(text).as_ptr()),
            WINDOW_STYLE(
                WS_CHILD.0
                    | WS_VISIBLE.0
                    | WS_BORDER.0
                    | WS_VSCROLL.0
                    | ES_MULTILINE as u32
                    | ES_AUTOVSCROLL as u32,
            ),
            x,
            y,
            width,
            height,
            hwnd,
            HMENU(id as *mut core::ffi::c_void),
            None,
            None,
        )
        .unwrap_or(HWND(null_mut()))
    }
}

fn create_list(hwnd: HWND, x: i32, y: i32, width: i32, height: i32, id: usize) -> HWND {
    unsafe {
        let list = CreateWindowExW(
            Default::default(),
            WC_LISTVIEWW,
            PCWSTR(to_wstring("").as_ptr()),
            WINDOW_STYLE(
                WS_CHILD.0
                    | WS_VISIBLE.0
                    | WS_BORDER.0
                    | WS_VSCROLL.0
                    | LVS_REPORT as u32
                    | LVS_SHOWSELALWAYS as u32,
            ),
            x,
            y,
            width,
            height,
            hwnd,
            HMENU(id as *mut core::ffi::c_void),
            None,
            None,
        )
        .unwrap_or(HWND(null_mut()));
        configure_list_view(list);
        list
    }
}

fn configure_list_view(list: HWND) {
    let extended_style = LVS_EX_CHECKBOXES | LVS_EX_FULLROWSELECT | LVS_EX_GRIDLINES;
    unsafe {
        let _ = SendMessageW(
            list,
            LVM_SETEXTENDEDLISTVIEWSTYLE,
            WPARAM(extended_style as usize),
            LPARAM(extended_style as isize),
        );
    }
    let columns = [
        ("选取", 54),
        ("ID", 96),
        ("状态", 76),
        ("标签", 150),
        ("标题", 170),
        ("内容预览", 360),
    ];
    for (index, (title, width)) in columns.iter().enumerate() {
        insert_list_view_column(list, index, title, *width);
    }
}

fn insert_list_view_column(list: HWND, index: usize, title: &str, width: i32) {
    let mut title = to_wstring(title);
    let mut column = LVCOLUMNW {
        mask: LVCF_FMT | LVCF_TEXT | LVCF_WIDTH,
        fmt: LVCFMT_LEFT,
        cx: width,
        pszText: windows::core::PWSTR(title.as_mut_ptr()),
        ..Default::default()
    };
    unsafe {
        let _ = SendMessageW(
            list,
            LVM_INSERTCOLUMNW,
            WPARAM(index),
            LPARAM((&mut column as *mut LVCOLUMNW) as isize),
        );
    }
}

fn create_button(
    hwnd: HWND,
    text: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    id: usize,
) -> HWND {
    unsafe {
        CreateWindowExW(
            Default::default(),
            PCWSTR(to_wstring("BUTTON").as_ptr()),
            PCWSTR(to_wstring(text).as_ptr()),
            WS_CHILD | WS_VISIBLE,
            x,
            y,
            width,
            height,
            hwnd,
            HMENU(id as *mut core::ffi::c_void),
            None,
            None,
        )
        .unwrap_or(HWND(null_mut()))
    }
}

fn create_status_switch(hwnd: HWND, x: i32, y: i32, width: i32, height: i32, id: usize) -> HWND {
    unsafe {
        CreateWindowExW(
            Default::default(),
            PCWSTR(to_wstring("RuleStatusSwitch").as_ptr()),
            PCWSTR(to_wstring("").as_ptr()),
            WS_CHILD | WS_VISIBLE,
            x,
            y,
            width,
            height,
            hwnd,
            HMENU(id as *mut core::ffi::c_void),
            None,
            None,
        )
        .unwrap_or(HWND(null_mut()))
    }
}

fn create_label(hwnd: HWND, text: &str, x: i32, y: i32, width: i32, height: i32) -> HWND {
    unsafe {
        CreateWindowExW(
            Default::default(),
            PCWSTR(to_wstring("STATIC").as_ptr()),
            PCWSTR(to_wstring(text).as_ptr()),
            WS_CHILD | WS_VISIBLE,
            x,
            y,
            width,
            height,
            hwnd,
            HMENU(null_mut()),
            None,
            None,
        )
        .unwrap_or(HWND(null_mut()))
    }
}

fn read_window_text(hwnd: HWND) -> String {
    unsafe {
        let len = windows::Win32::UI::WindowsAndMessaging::GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return String::new();
        }
        let mut buffer = vec![0u16; len as usize + 1];
        let read = windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(hwnd, &mut buffer);
        if read <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buffer[..read as usize])
    }
}

fn set_window_text(hwnd: HWND, text: &str) {
    unsafe {
        let _ = SetWindowTextW(hwnd, PCWSTR(to_wstring(text).as_ptr()));
    }
}

fn split_tags(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .fold(Vec::new(), |mut tags, tag| {
            if !tags.iter().any(|existing| existing == tag) {
                tags.push(tag.to_string());
            }
            tags
        })
}

fn apply_layout(hwnd: HWND, state: &WindowState) {
    let mut rect = RECT::default();
    unsafe {
        let _ = GetClientRect(hwnd, &mut rect);
    }
    let width = (rect.right - rect.left).max(720);
    let height = (rect.bottom - rect.top).max(520);
    let pad = 18;
    let button_w = 150;
    let button_h = 36;
    let button_y = height - pad - button_h;
    let confirm_w = 170;
    let confirm_x = width - pad - confirm_w;
    let cancel_x = confirm_x - 18 - button_w;
    let editor_x = (width * 3 / 5).max(430);
    let editor_w = (width - editor_x - pad).max(260);
    let list_w = (editor_x - pad * 2).max(260);
    let list_top = 154;
    let editor_top = 66;
    let save_y = button_y - 44;
    let status_edit_y = save_y - 42;
    let status_label_y = status_edit_y - 24;
    let tags_edit_y = status_label_y - 42;
    let tags_label_y = tags_edit_y - 24;
    let content_y = 226;
    let content_h = (tags_label_y - content_y - 12).max(96);

    set_bounds(state.title_label, pad, 18, 220, 32);
    set_bounds(state.hint_label, pad + 234, 24, width - pad * 2 - 234, 24);
    set_bounds(state.search_label, pad, 66, list_w, 22);
    set_bounds(state.search, pad, 90, list_w, 30);
    set_bounds(state.list_status_label, pad, 128, list_w, 22);
    set_bounds(state.list, pad, list_top, list_w, button_y - list_top - 12);
    set_bounds(
        state.editor_heading_label,
        editor_x,
        editor_top,
        editor_w,
        28,
    );
    set_bounds(
        state.editor_hint_label,
        editor_x,
        editor_top + 30,
        editor_w,
        22,
    );
    set_bounds(state.title_field_label, editor_x, 132, editor_w, 22);
    set_bounds(state.title_edit, editor_x, 156, editor_w, 30);
    set_bounds(state.content_field_label, editor_x, 202, editor_w, 22);
    set_bounds(state.content_edit, editor_x, content_y, editor_w, content_h);
    set_bounds(state.tags_field_label, editor_x, tags_label_y, editor_w, 22);
    set_bounds(state.tags_edit, editor_x, tags_edit_y, editor_w, 30);
    set_bounds(
        state.status_field_label,
        editor_x,
        status_label_y,
        editor_w,
        22,
    );
    set_bounds(state.status_switch, editor_x, status_edit_y, 248, 34);
    // 保存编辑是“编辑当前行”的局部动作，取消/确认是窗口级动作。
    // 分成两行能避免窄窗口下三按钮互相挤压，也让操作层级更清楚。
    set_bounds(state.save_button, editor_x, save_y, button_w, button_h);
    set_bounds(state.cancel_button, cancel_x, button_y, button_w, button_h);
    set_bounds(
        state.confirm_button,
        confirm_x,
        button_y,
        confirm_w,
        button_h,
    );
}

fn set_bounds(hwnd: HWND, x: i32, y: i32, width: i32, height: i32) {
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            HWND(null_mut()),
            x,
            y,
            width.max(1),
            height.max(1),
            SWP_NOZORDER,
        );
    }
}

fn set_font(hwnd: HWND, font: HFONT) {
    unsafe {
        let _ = SendMessageW(hwnd, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));
    }
}

fn to_wstring(input: &str) -> Vec<u16> {
    OsStr::new(input)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn loword(value: u32) -> u16 {
    (value & 0xFFFF) as u16
}

fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
}
