//! Symbol-based filtering to classify functions as user code vs runtime/CRT.

use crate::analysis::Function;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionClass {
    UserCode,
    RuntimeCrt,
}

/// Common CRT/runtime function name prefixes (MinGW, MSVC, GNU, POSIX).
const CRT_PREFIXES: &[&str] = &[
    "__mingw_",      // MinGW-specific CRT functions
    "_mingw_",       // Alternative MinGW prefix
    "__crt_",        // Generic CRT prefix
    "_crt_",         // Alternative CRT prefix
    "__gnu_",        // GNU-specific functions
    "_gnu_",         // Alternative GNU prefix
    "__p_",          // CRT pointers (locale, commode, etc.)
    "__C_",          // MSVC C runtime internals
    "_initterm",     // CRT initialization
    "_pei386_",      // PE-specific runtime relocation
    "__do_global_",  // Global constructor/destructor
    "__dyn_tls_",    // TLS dynamic initialization
    "__gcc_",        // GCC internal functions
    "__acrt_",       // CRT internal (MSVC)
    "__iob_",        // MSVC I/O buffer functions
    "__setusermatherr", // MSVC math error handlers
    "__chkstk",      // Stack checking (compiler-generated)
    "abort",         // CRT stubs
    "exit",
    "atexit",
    "calloc",
    "malloc",
    "realloc",
    "free",
    "memcpy",
    "strlen",
    "strncmp",
    "fprintf",
    "printf",        // Note: just "printf" without prefix
    "vfprintf",
    "sprintf",
    "vsprintf",
    "fputc",
    "putchar",
    "rand",
    "srand",
    "signal",
    "sleep",
    "localtime",
    "time",
    "_time",
    "localeconv",
    "mbrtowc",
    "wcrtomb",
    "VirtualProtect",
    "VirtualQuery",
    "GetLastError",
    "SetUnhandledExceptionFilter",
    "EnterCriticalSection",
    "LeaveCriticalSection",
    "InitializeCriticalSection",
    "DeleteCriticalSection",
    "TlsGetValue",
    "Sleep",
    "MultiByteToWideChar",
    "WideCharToMultiByte",
    // Double-to-ASCII conversion (printf internals)
    "__gdtoa",
    "__b2d_D2A",
    "__Balloc_D2A",
    "__Bfree_D2A",
    "__multadd_D2A",
    "__mult_D2A",
    "__i2b_D2A",
    "__d2b_D2A",
    "__diff_D2A",
    "__rshift_D2A",
    "__lshift_D2A",
    "__pow5mult_D2A",
    "__rv_alloc_D2A",
    "__nrv_alloc_D2A",
    "__quorem_D2A",
    "__freedtoa",
    "__strcp_D2A",
];

/// Classify a function based on its name.
pub fn classify(func_name: &str) -> FunctionClass {
    // Check against known CRT prefixes/names
    for prefix in CRT_PREFIXES {
        if func_name.starts_with(prefix) {
            return FunctionClass::RuntimeCrt;
        }
    }

    // Check for common CRT patterns (all caps or leading _)
    if func_name.starts_with("main") || func_name.starts_with("WinMain") {
        return FunctionClass::UserCode; // Entry points are user code
    }

    // Default to user code
    FunctionClass::UserCode
}

/// Load a .symbols file (from nm output).
/// Format: "address T name" (one per line, already filtered to T entries)
pub fn load_symbols_file(path: &Path) -> std::io::Result<BTreeSet<String>> {
    let content = std::fs::read_to_string(path)?;
    let mut symbols = BTreeSet::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 && parts[1] == "T" {
            symbols.insert(parts[2].to_string());
        }
    }
    Ok(symbols)
}

/// Filter functions by class.
pub fn filter_by_class(functions: &[Function], class: FunctionClass) -> Vec<&Function> {
    functions
        .iter()
        .filter(|f| classify(&f.name) == class)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify() {
        assert_eq!(classify("insertionSort"), FunctionClass::UserCode);
        assert_eq!(classify("bubbleSort"), FunctionClass::UserCode);
        assert_eq!(classify("main"), FunctionClass::UserCode);
        assert_eq!(classify("__mingw_printf"), FunctionClass::RuntimeCrt);
        assert_eq!(classify("__gdtoa"), FunctionClass::RuntimeCrt);
        assert_eq!(classify("malloc"), FunctionClass::RuntimeCrt);
        assert_eq!(classify("printf"), FunctionClass::RuntimeCrt);
    }
}
