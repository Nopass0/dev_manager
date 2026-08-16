//! Math library (Rust) — part of multi-language build example.

/// Adds two numbers.
#[no_mangle]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// Multiplies two numbers.
#[no_mangle]
pub extern "C" fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

/// Calculates factorial.
#[no_mangle]
pub extern "C" fn factorial(n: u32) -> u64 {
    match n {
        0 | 1 => 1,
        _ => (1..=n as u64).product(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
    }

    #[test]
    fn test_multiply() {
        assert_eq!(multiply(4, 5), 20);
    }

    #[test]
    fn test_factorial() {
        assert_eq!(factorial(5), 120);
        assert_eq!(factorial(0), 1);
    }
}
