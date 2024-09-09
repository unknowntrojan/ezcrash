use std::{cell::OnceCell, ffi::CString, io::Write};
use strum::{Display, FromRepr};
use windows::{
    core::{s, PCSTR},
    Win32::{
        Foundation::HWND,
        System::Diagnostics::Debug::{
            AddVectoredExceptionHandler, EXCEPTION_CONTINUE_EXECUTION, EXCEPTION_CONTINUE_SEARCH,
            EXCEPTION_POINTERS,
        },
        UI::WindowsAndMessaging::{MessageBoxA, MB_OK},
    },
};

#[repr(u32)]
#[allow(non_camel_case_types)]
#[derive(Debug, FromRepr, Display)]
pub enum ExceptionType {
    EXCEPTION_ACCESS_VIOLATION = 0xC0000005,
    EXCEPTION_ARRAY_BOUNDS_EXCEEDED = 0xC000008C,
    EXCEPTION_BREAKPOINT = 0x80000003,
    EXCEPTION_DATATYPE_MISALIGNMENT = 0x80000002,
    EXCEPTION_FLT_DENORMAL_OPERAND = 0xC000008D,
    EXCEPTION_FLT_DIVIDE_BY_ZERO = 0xC000008E,
    EXCEPTION_FLT_INEXACT_RESULT = 0xC000008F,
    EXCEPTION_FLT_INVALID_OPERATION = 0xC0000090,
    EXCEPTION_FLT_OVERFLOW = 0xC0000091,
    EXCEPTION_FLT_STACK_CHECK = 0xC0000092,
    EXCEPTION_FLT_UNDERFLOW = 0xC0000093,
    EXCEPTION_GUARD_PAGE = 0x80000001,
    EXCEPTION_ILLEGAL_INSTRUCTION = 0xC000001D,
    EXCEPTION_INT_DIVIDE_BY_ZERO = 0xC0000094,
    EXCEPTION_INT_OVERFLOW = 0xC0000095,
    EXCEPTION_INVALID_DISPOSITION = 0xC0000026,
    EXCEPTION_INVALID_HANDLE = 0xC0000008,
    EXCEPTION_IN_PAGE_ERROR = 0xC0000006,
    EXCEPTION_NONCONTINUABLE_EXCEPTION = 0xC0000025,
    EXCEPTION_POSSIBLE_DEADLOCK = 0xC0000194,
    EXCEPTION_PRIV_INSTRUCTION = 0xC0000096,
    EXCEPTION_SINGLE_STEP = 0x80000004,
    EXCEPTION_SPAPI_UNRECOVERABLE_STACK_OVERFLOW = 0xE0000300,
    EXCEPTION_STACK_OVERFLOW = 0xC00000FD,
}

unsafe extern "system" fn handler(ptrs: *mut EXCEPTION_POINTERS) -> i32 {
    let cfg = CFG
        .get()
        .map_or(EzCrashConfiguration::default(), |x| x.clone());

    let mut w = Vec::new();
    let _ = writeln!(&mut w, "Crash :(");

    let ctx = &mut *(*ptrs).ContextRecord;
    let record = &mut *(*ptrs).ExceptionRecord;

    let Some(code) = ExceptionType::from_repr(record.ExceptionCode.0 as _) else {
        // no
        return EXCEPTION_CONTINUE_EXECUTION;
    };

    let addr = record.ExceptionAddress as usize;

    let info = (
        record.ExceptionInformation[0],
        record.ExceptionInformation[1],
        record.ExceptionInformation[2],
    );

    let _ = writeln!(&mut w, "An exception occurred at {:#018X}\n", addr);
    let _ = writeln!(
        &mut w,
        "{:#010X}: {}\n",
        record.ExceptionCode.0 as u32, code
    );

    match code {
        ExceptionType::EXCEPTION_ACCESS_VIOLATION | ExceptionType::EXCEPTION_IN_PAGE_ERROR => {
            let _ = match info.0 {
                0 => writeln!(&mut w, "Invalid Read from {:#018X}", info.1),
                1 => writeln!(&mut w, "Invalid Write to {:#018X}", info.1),
                8 => writeln!(&mut w, "Tripped DEP at {:#018X}", info.1),
                _ => writeln!(&mut w, "Unknown access violation at {:#018X}", info.1),
            };
        }
        _ => {}
    }

    if cfg.include_thread_context {
        let _ = writeln!(
            &mut w,
            "\nThread Context\nRAX: {:#018X} | RSI: {:#018X}\nRBX: {:#018X} | RDI: {:#018X}",
            ctx.Rax, ctx.Rsi, ctx.Rbx, ctx.Rdi
        );
        let _ = writeln!(
            &mut w,
            "RCX: {:#018X} | RBP: {:#018X}\nRDX: {:#018X} | RSP: {:#018X}",
            ctx.Rcx, ctx.Rbp, ctx.Rdx, ctx.Rsp
        );
        let _ = writeln!(
            &mut w,
            "R8: {:#018X} | R9: {:#018X}\nR10: {:#018X} | R11: {:#018X}",
            ctx.R8, ctx.R9, ctx.R10, ctx.R11
        );
        let _ = writeln!(
            &mut w,
            "R12: {:#018X} | R13: {:#018X}\nR14: {:#018X} | R15: {:#018X}\n\nRIP: {:#018X}",
            ctx.R12, ctx.R13, ctx.R14, ctx.R15, ctx.Rip
        );
    }

    let backtrace = std::backtrace::Backtrace::force_capture();

    if cfg.include_stack_trace {
        let _ = writeln!(&mut w, "\nStack Trace\n{}", backtrace);
    }

    if let Some(path) = cfg.output_file {
        let _ = std::fs::write(path, &w);
    }

    let message = CString::new(w).unwrap();

    if cfg.output_log {
        log::error!("{}", message.to_string_lossy());
    }

    if cfg.output_messagebox {
        MessageBoxA(
            HWND(0 as _),
            PCSTR(message.as_ptr() as _),
            s!("Crash"),
            MB_OK,
        );
    }

    EXCEPTION_CONTINUE_SEARCH
}

#[derive(Clone)]
pub struct EzCrashConfiguration {
    pub output_messagebox: bool,
    pub output_log: bool,
    pub output_file: Option<String>,
    pub include_stack_trace: bool,
    pub include_thread_context: bool,
}

impl Default for EzCrashConfiguration {
    fn default() -> Self {
        Self {
            output_messagebox: true,
            output_log: true,
            output_file: Some(String::from("crash")),
            include_stack_trace: true,
            include_thread_context: true,
        }
    }
}

static mut CFG: OnceCell<EzCrashConfiguration> = OnceCell::new();

/// Adds a Vectored Exception Handler to your program.
pub fn init(cfg: EzCrashConfiguration) {
    let _ = unsafe { CFG.set(cfg) };

    let _ = unsafe { AddVectoredExceptionHandler(0, Some(handler)) };
}

// #[test]
// fn test() {
//     init(EzCrashConfiguration {
//         output_messagebox: true,
//         output_log: true,
//         output_file: Some(String::from("crash")),
//         include_stack_trace: true,
//         include_thread_context: true,
//     });

//     print!("result: {}", 8 + unsafe { *std::ptr::null::<usize>() });
// }
