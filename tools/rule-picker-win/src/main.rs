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
    ES_AUTOHSCROLL, FVIRTKEY, GWLP_USERDATA, GetClientRect, GetMessageW, GetWindowLongPtrW, HMENU,
    LB_ADDSTRING, LB_GETSEL, LB_RESETCONTENT, LB_SETSEL, LBS_EXTENDEDSEL, LBS_NOTIFY, LoadCursorW,
    MSG, PostQuitMessage, RegisterClassW, SW_RESTORE, SWP_NOZORDER, SendMessageW,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, TranslateAcceleratorW, TranslateMessage,
    WINDOW_STYLE, WM_CLOSE, WM_COMMAND, WM_CREATE, WM_CTLCOLORDLG, WM_CTLCOLOREDIT,
    WM_CTLCOLORLISTBOX, WM_CTLCOLORSTATIC, WM_DESTROY, WM_ERASEBKGND, WM_SETFONT, WM_SIZE,
    WNDCLASSW, WS_BORDER, WS_CHILD, WS_OVERLAPPEDWINDOW, WS_VISIBLE, WS_VSCROLL,
};
use windows::core::PCWSTR;

const ID_SEARCH: usize = 1001;
const ID_LIST: usize = 1002;
const ID_CONFIRM: usize = 1003;
const ID_CANCEL: usize = 1004;

#[derive(Clone, Debug, Deserialize)]
struct RuleInput {
    id: String,
    title: String,
    content: String,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Clone, Debug)]
struct RuleItem {
    id: String,
    display: String,
    search_text: String,
}

#[derive(Debug)]
enum PickerAction {
    Pick(Vec<String>),
    Cancel,
}

#[derive(Serialize)]
struct PickerOutput {
    selected_ids: Vec<String>,
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
    confirm_button: HWND,
    cancel_button: HWND,
    bg_brush: HBRUSH,
    input_brush: HBRUSH,
    font: HFONT,
    action_sent: bool,
}

fn main() {
    let result = run();
    match result {
        Ok(output) => {
            println!(
                "{}",
                serde_json::to_string(&output).unwrap_or_else(|_| {
                    "{\"selected_ids\":[],\"cancelled\":true}".to_string()
                })
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
    if rules.is_empty() {
        return Ok(PickerOutput {
            selected_ids: Vec::new(),
            cancelled: true,
        });
    }

    match run_picker_window(rules, args.query) {
        PickerAction::Pick(selected_ids) => Ok(PickerOutput {
            selected_ids,
            cancelled: false,
        }),
        PickerAction::Cancel => Ok(PickerOutput {
            selected_ids: Vec::new(),
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
                    return Err("Usage: rule-picker-win --rules <rules.json> [--query <text>]"
                        .to_string());
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
            cancelled: true,
        });
    }
    let ids = env::var("RULE_PICKER_HEADLESS_IDS").ok()?;
    Some(PickerOutput {
        selected_ids: split_ids(&ids),
        cancelled: false,
    })
}

fn truthy_env(name: &str) -> bool {
    env::var(name)
        .map(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
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
    let raw_rules: Vec<RuleInput> = serde_json::from_str(&content).map_err(|err| err.to_string())?;
    let rules = raw_rules
        .into_iter()
        .filter_map(|rule| {
            let id = rule.id.trim().to_string();
            let title = rule.title.trim().to_string();
            let content = rule.content.trim().to_string();
            if id.is_empty() || title.is_empty() || content.is_empty() {
                return None;
            }

            let tags = rule
                .tags
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(",");
            let display = format!("{id} [{tags}] {title}: {content}");
            let search_text = format!(
                "{} {} {} {}",
                id.to_ascii_lowercase(),
                title.to_lowercase(),
                content.to_lowercase(),
                tags.to_lowercase()
            );
            Some(RuleItem {
                id,
                display,
                search_text,
            })
        })
        .collect::<Vec<_>>();
    Ok(rules)
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

            let _label = create_label(hwnd, "输入关键词过滤，勾选规则后按 Enter 确认，Esc 取消。", 18, 18, 720, 24);
            let search = create_edit(hwnd, &params.initial_query, 18, 50, 924, 30, ID_SEARCH);
            let list = create_list(hwnd, 18, 92, 924, 420, ID_LIST);
            let cancel_button = create_button(hwnd, "取消", 642, 530, 140, 34, ID_CANCEL);
            let confirm_button = create_button(hwnd, "选取规则", 802, 530, 140, 34, ID_CONFIRM);

            set_font(search, font);
            set_font(list, font);
            set_font(cancel_button, font);
            set_font(confirm_button, font);

            let mut state = Box::new(WindowState {
                sender: params.sender,
                rules: params.rules,
                visible_indices: Vec::new(),
                search,
                list,
                confirm_button,
                cancel_button,
                bg_brush,
                input_brush,
                font,
                action_sent: false,
            });
            refresh_visible_rules(&mut state);
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
                ID_SEARCH => refresh_visible_rules(state),
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
                unsafe { let _ = DestroyWindow(hwnd); }
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

    if state.visible_indices.len() == 1 {
        unsafe {
            let _ = SendMessageW(state.list, LB_SETSEL, WPARAM(1), LPARAM(0));
        }
    }
}

fn finish_with_selection(hwnd: HWND, state: &mut WindowState) {
    let mut selected_ids = Vec::new();
    for (visible_index, rule_index) in state.visible_indices.iter().enumerate() {
        let selected = unsafe {
            SendMessageW(state.list, LB_GETSEL, WPARAM(visible_index), LPARAM(0)).0 > 0
        };
        if selected {
            selected_ids.push(state.rules[*rule_index].id.clone());
        }
    }

    // 搜索后只剩一条时允许直接 Enter，减少“还得点一下”的摩擦。
    if selected_ids.is_empty() && state.visible_indices.len() == 1 {
        selected_ids.push(state.rules[state.visible_indices[0]].id.clone());
    }

    state.action_sent = true;
    let _ = state.sender.send(PickerAction::Pick(selected_ids));
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
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

fn create_button(hwnd: HWND, text: &str, x: i32, y: i32, width: i32, height: i32, id: usize) -> HWND {
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

    set_bounds(state.search, pad, 50, width - pad * 2, 30);
    set_bounds(state.list, pad, 92, width - pad * 2, button_y - 108);
    set_bounds(state.cancel_button, cancel_x, button_y, button_w, button_h);
    set_bounds(state.confirm_button, confirm_x, button_y, button_w, button_h);
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
