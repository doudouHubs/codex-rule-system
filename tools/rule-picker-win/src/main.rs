use serde::{Deserialize, Serialize};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::ptr::null_mut;
use std::sync::mpsc::{Sender, channel};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CLIP_DEFAULT_PRECIS, CreateFontW, CreateSolidBrush, DEFAULT_CHARSET, DEFAULT_QUALITY,
    DeleteObject, FF_DONTCARE, FW_NORMAL, FillRect, HBRUSH, HDC, HFONT, OUT_DEFAULT_PRECIS,
    SetBkColor, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{SetFocus, VK_ESCAPE, VK_RETURN};
use windows::Win32::UI::WindowsAndMessaging::{
    ACCEL, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateAcceleratorTableW,
    CreateWindowExW, DefWindowProcW, DestroyAcceleratorTable, DestroyWindow, DispatchMessageW,
    ES_AUTOHSCROLL, ES_AUTOVSCROLL, ES_MULTILINE, FVIRTKEY, GWLP_USERDATA, GetClientRect,
    GetMessageW, GetWindowLongPtrW, HMENU, LB_ADDSTRING, LB_GETCURSEL, LB_GETSEL, LB_RESETCONTENT,
    LB_SETSEL, LBN_SELCHANGE, LBS_EXTENDEDSEL, LBS_NOTIFY, LoadCursorW, MSG, PostQuitMessage,
    RegisterClassW, SW_RESTORE, SWP_NOZORDER, SendMessageW, SetWindowLongPtrW, SetWindowPos,
    SetWindowTextW, ShowWindow, TranslateAcceleratorW, TranslateMessage, WINDOW_STYLE, WM_CLOSE,
    WM_COMMAND, WM_CREATE, WM_CTLCOLORDLG, WM_CTLCOLOREDIT, WM_CTLCOLORLISTBOX, WM_CTLCOLORSTATIC,
    WM_DESTROY, WM_ERASEBKGND, WM_SETFONT, WM_SIZE, WNDCLASSW, WS_BORDER, WS_CHILD,
    WS_OVERLAPPEDWINDOW, WS_VISIBLE, WS_VSCROLL,
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
    search: HWND,
    list: HWND,
    title_edit: HWND,
    content_edit: HWND,
    tags_edit: HWND,
    status_edit: HWND,
    save_button: HWND,
    confirm_button: HWND,
    cancel_button: HWND,
    bg_brush: HBRUSH,
    input_brush: HBRUSH,
    font: HFONT,
    action_sent: bool,
    editing_rule_index: Option<usize>,
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

        let params = Box::new(CreateParams {
            sender,
            rules,
            initial_query,
        });
        let hwnd = CreateWindowExW(
            Default::default(),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(to_wstring("Rule Picker").as_ptr()),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            980,
            620,
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

            let _label = create_label(
                hwnd,
                "搜索并勾选规则；选中单条后可编辑项目规则。Enter 保存编辑并选取，Esc 取消。",
                18,
                18,
                900,
                24,
            );
            let search = create_edit(hwnd, &params.initial_query, 18, 50, 560, 30, ID_SEARCH);
            let list = create_list(hwnd, 18, 92, 560, 420, ID_LIST);
            let _title_label = create_label(hwnd, "标题", 598, 50, 340, 22);
            let title_edit = create_edit(hwnd, "", 598, 74, 344, 30, ID_TITLE);
            let _content_label = create_label(hwnd, "内容", 598, 112, 340, 22);
            let content_edit = create_multiline_edit(hwnd, "", 598, 136, 344, 190, ID_CONTENT);
            let _tags_label = create_label(hwnd, "标签（逗号分隔）", 598, 342, 340, 22);
            let tags_edit = create_edit(hwnd, "", 598, 366, 344, 30, ID_TAGS);
            let _status_label = create_label(hwnd, "状态（active/deprecated）", 598, 404, 340, 22);
            let status_edit = create_edit(hwnd, "active", 598, 428, 344, 30, ID_STATUS);
            let save_button = create_button(hwnd, "保存编辑", 598, 478, 140, 34, ID_SAVE_EDIT);
            let cancel_button = create_button(hwnd, "取消", 642, 530, 140, 34, ID_CANCEL);
            let confirm_button = create_button(hwnd, "保存并选取", 802, 530, 140, 34, ID_CONFIRM);

            set_font(search, font);
            set_font(list, font);
            set_font(title_edit, font);
            set_font(content_edit, font);
            set_font(tags_edit, font);
            set_font(status_edit, font);
            set_font(save_button, font);
            set_font(cancel_button, font);
            set_font(confirm_button, font);

            let mut state = Box::new(WindowState {
                sender: params.sender,
                rules: params.rules,
                visible_indices: Vec::new(),
                search,
                list,
                title_edit,
                content_edit,
                tags_edit,
                status_edit,
                save_button,
                confirm_button,
                cancel_button,
                bg_brush,
                input_brush,
                font,
                action_sent: false,
                editing_rule_index: None,
            });
            refresh_visible_rules(&mut state);
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
            let notification = hiword(wparam.0 as u32);
            let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState };
            if ptr.is_null() {
                return LRESULT(0);
            }
            let state = unsafe { &mut *ptr };
            match id {
                ID_SEARCH => {
                    save_editor_to_current_rule(state);
                    refresh_visible_rules(state);
                    load_editor_from_current_selection(state);
                }
                ID_LIST if notification as u32 == LBN_SELCHANGE => {
                    save_editor_to_current_rule(state);
                    load_editor_from_current_selection(state);
                }
                ID_SAVE_EDIT => {
                    save_editor_to_current_rule(state);
                    refresh_visible_rules(state);
                    load_editor_from_current_selection(state);
                }
                ID_CONFIRM => finish_with_selection(hwnd, state),
                ID_CANCEL => finish_cancelled(hwnd, state),
                _ => {}
            }
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
            unsafe {
                SetBkMode(hdc, TRANSPARENT);
                SetTextColor(hdc, rgb(48, 52, 58));
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
                }
            }
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
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
        let _ = SendMessageW(state.list, LB_RESETCONTENT, WPARAM(0), LPARAM(0));
    }

    for (index, rule) in state.rules.iter().enumerate() {
        if !terms.iter().all(|term| rule.search_text.contains(term)) {
            continue;
        }
        state.visible_indices.push(index);
        let text = to_wstring(&rule.display);
        unsafe {
            let _ = SendMessageW(
                state.list,
                LB_ADDSTRING,
                WPARAM(0),
                LPARAM(text.as_ptr() as isize),
            );
        }
    }

    if state.visible_indices.is_empty() {
        let message = if state.rules.is_empty() {
            "当前无 active 项目规则。请先用 rule-add --scope project 新增。"
        } else {
            "没有匹配的项目规则。请调整搜索关键词。"
        };
        let text = to_wstring(message);
        unsafe {
            let _ = SendMessageW(
                state.list,
                LB_ADDSTRING,
                WPARAM(0),
                LPARAM(text.as_ptr() as isize),
            );
        }
    }

    if state.visible_indices.len() == 1 {
        unsafe {
            let _ = SendMessageW(state.list, LB_SETSEL, WPARAM(1), LPARAM(0));
        }
    }
}

fn finish_with_selection(hwnd: HWND, state: &mut WindowState) {
    save_editor_to_current_rule(state);

    let mut selected_ids = Vec::new();
    for (visible_index, rule_index) in state.visible_indices.iter().enumerate() {
        let selected =
            unsafe { SendMessageW(state.list, LB_GETSEL, WPARAM(visible_index), LPARAM(0)).0 > 0 };
        if selected {
            selected_ids.push(state.rules[*rule_index].id.clone());
        }
    }

    // 搜索后只剩一条时允许直接 Enter，减少“还得点一下”的摩擦。
    if selected_ids.is_empty() && state.visible_indices.len() == 1 {
        selected_ids.push(state.rules[state.visible_indices[0]].id.clone());
    }

    state.action_sent = true;
    let _ = state.sender.send(PickerAction::Pick(PickerResult {
        selected_ids,
        updates: collect_updates(state),
    }));
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

fn load_editor_from_current_selection(state: &mut WindowState) {
    let visible_index = current_visible_index(state);
    let Some(visible_index) = visible_index else {
        state.editing_rule_index = None;
        set_window_text(state.title_edit, "");
        set_window_text(state.content_edit, "");
        set_window_text(state.tags_edit, "");
        set_window_text(state.status_edit, "active");
        return;
    };
    let Some(rule_index) = state.visible_indices.get(visible_index).copied() else {
        state.editing_rule_index = None;
        return;
    };
    let rule = &state.rules[rule_index];
    state.editing_rule_index = Some(rule_index);
    set_window_text(state.title_edit, &rule.title);
    set_window_text(state.content_edit, &rule.content);
    set_window_text(state.tags_edit, &rule.tags.join(","));
    set_window_text(state.status_edit, &rule.status);
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
    rule.status = normalize_status(&read_window_text(state.status_edit));
    refresh_rule_text(rule);
}

fn current_visible_index(state: &WindowState) -> Option<usize> {
    if state.visible_indices.is_empty() {
        return None;
    }
    let raw_index = unsafe { SendMessageW(state.list, LB_GETCURSEL, WPARAM(0), LPARAM(0)).0 };
    if raw_index >= 0 {
        let index = raw_index as usize;
        if index < state.visible_indices.len() {
            return Some(index);
        }
    }
    if state.visible_indices.len() == 1 {
        return Some(0);
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
        CreateWindowExW(
            Default::default(),
            PCWSTR(to_wstring("LISTBOX").as_ptr()),
            PCWSTR(to_wstring("").as_ptr()),
            WINDOW_STYLE(
                WS_CHILD.0
                    | WS_VISIBLE.0
                    | WS_BORDER.0
                    | WS_VSCROLL.0
                    | LBS_EXTENDEDSEL as u32
                    | LBS_NOTIFY as u32,
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
    let width = (rect.right - rect.left).max(480);
    let height = (rect.bottom - rect.top).max(320);
    let pad = 18;
    let button_w = 140;
    let button_h = 34;
    let button_y = height - pad - button_h;
    let confirm_x = width - pad - button_w;
    let cancel_x = confirm_x - 18 - button_w;
    let editor_x = (width * 3 / 5).max(360);
    let editor_w = (width - editor_x - pad).max(260);
    let list_w = (editor_x - pad * 2).max(260);

    set_bounds(state.search, pad, 50, list_w, 30);
    set_bounds(state.list, pad, 92, list_w, button_y - 108);
    set_bounds(state.title_edit, editor_x, 74, editor_w, 30);
    set_bounds(
        state.content_edit,
        editor_x,
        136,
        editor_w,
        (button_y - 262).max(120),
    );
    set_bounds(state.tags_edit, editor_x, button_y - 164, editor_w, 30);
    set_bounds(state.status_edit, editor_x, button_y - 102, editor_w, 30);
    set_bounds(
        state.save_button,
        editor_x,
        button_y - 52,
        button_w,
        button_h,
    );
    set_bounds(state.cancel_button, cancel_x, button_y, button_w, button_h);
    set_bounds(
        state.confirm_button,
        confirm_x,
        button_y,
        button_w,
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

fn hiword(value: u32) -> u16 {
    ((value >> 16) & 0xFFFF) as u16
}

fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
}
