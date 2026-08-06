//! Утилиты для работы с сетью: корректная проверка занятости порта.
//!
//! Важно: [`port_is_free`] использует реальную попытку `bind` (занять порт),
//! а не `connect`. Это даёт точный ответ «порт свободен ли для прослушивания» —
//! в отличие от `connect`, который лжёт на TIME_WAIT и при partial-accept.

use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};

/// Возвращает true, если порт свободен (можно начать слушать).
///
/// Использует `TcpListener::bind` — реальную попытку занять порт. Если bind
/// успешен, порт свободен (мы тут же освобождаем его, закрывая listener).
/// Это корректнее `connect`, который не видит порты в TIME_WAIT или считает
/// «занятым» порт, который сервис ещё не успел начать слушать.
pub fn port_is_free(port: u16) -> bool {
    let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
    TcpListener::bind(addr).is_ok()
}

/// Возвращает true, если порт занят (кто-то уже слушает его).
pub fn port_is_in_use(port: u16) -> bool {
    !port_is_free(port)
}

/// Ищет первый свободный порт начиная с `start` до `end` (включительно).
///
/// Полезно для авто-подбора порта при конфликтах. Возвращает None, если все
/// порты в диапазоне заняты.
pub fn find_free_port(start: u16, end: u16) -> Option<u16> {
    (start..=end).find(|&port| port_is_free(port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_port_is_free() {
        // Эфемерный порт почти наверняка свободен.
        assert!(port_is_free(0) || port_is_free(59999));
    }

    #[test]
    fn find_free_port_works() {
        // В широком диапазоне точно найдётся свободный.
        assert!(find_free_port(30000, 40000).is_some());
    }

    #[test]
    fn occupied_port_detected() {
        // Занимаем порт сами и проверяем, что он определяется как занятый.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(port_is_in_use(port));
        drop(listener);
    }
}
