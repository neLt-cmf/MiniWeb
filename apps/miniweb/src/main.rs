#![cfg_attr(any(target_os = "none", wp2_sim), no_main, cfg_attr(target_os = "none", no_std))]

#[cfg(not(any(target_os = "none", wp2_sim)))]
fn main() {}

#[cfg(any(target_os = "none", wp2_sim))]
mod device {
    use cmf_wp2_app::{Api, BUTTON, CROWN, PAUSE, RESUME, TOUCH_DOWN};
    use companion_rpc::{Call, Got, http_get, HttpHead, utf8_cut};

    cmf_wp2_app::entry!(main);

    const WHITE: u16 = 0xFFFF;
    const BLACK: u16 = 0x0000;
    const BLUE: u16 = 0x001F;
    const GREY: u16 = 0x8410;

    const CHARS: &[u8] =
        b"abcdefghijklmnopqrstuvwxyz0123456789.-_/:";

    fn draw(api: &Api, url: &[u8], selected: usize, loading: bool,
            code: u16, body: &[u8], body_len: usize) {
        api.rect(0, 0, api.width, api.height, BLACK);

        api.text(18, 24, "MiniWeb", WHITE);
        api.text(18, 52, "Address:", GREY);

        let address = core::str::from_utf8(url).unwrap_or("");
        api.text(18, 76, address, WHITE);

        api.rect(12, 112, 442, 2, BLUE);

        api.text(18, 132, "CROWN: choose character", GREY);
        api.text(18, 158, "TAP: add character", GREY);

        let ch = CHARS[selected] as char;
        let mut symbol = [0u8; 1];
        symbol[0] = ch as u8;
        let symbol_text = core::str::from_utf8(&symbol).unwrap_or("");
        api.text(220, 202, symbol_text, WHITE);

        api.rect(12, 250, 442, 2, GREY);
        api.text(18, 266, "Page:", WHITE);

        if loading {
            api.text(18, 292, "Loading...", BLUE);
        } else if code != 0 {
            let mut digits = [b' '; 3];
            digits[0] = b'0' + ((code / 100) % 10) as u8;
            digits[1] = b'0' + ((code / 10) % 10) as u8;
            digits[2] = b'0' + (code % 10) as u8;
            let status = core::str::from_utf8(&digits).unwrap_or("");
            api.text(18, 292, status, BLUE);

            let n = body_len.min(64);
            let preview = utf8_cut(&body[..n], 64);
            api.text(18, 320, preview, WHITE);
        } else {
            api.text(18, 292, "Enter address and tap OPEN", GREY);
        }

        api.rect(12, 390, 442, 54, BLUE);
        api.text(190, 408, "OPEN", WHITE);

        api.show();
    }

    fn main(api: &'static Api) -> i32 {
        let mut url = [0u8; 123];
        let initial = b"https://example.com";
        url[..initial.len()].copy_from_slice(initial);
        let mut url_len = initial.len();

        let mut selected = 0usize;

        let mut call = Call::new(30_000);
        let mut ids = 0u8;
        let mut loading = false;

        let mut body = [0u8; 2048];
        let mut body_len = 0usize;
        let mut expected_len = 0usize;
        let mut http_code = 0u16;

        let mut msg_buf = [0u8; 128];

        loop {
            if let Some(id) = call.due(api.now(), &mut ids) {
                let address = core::str::from_utf8(&url[..url_len]).unwrap_or("");
                if let Some(request) = http_get(id, address) {
                    api.send(request.as_bytes());
                } else {
                    loading = false;
                    call.stop();
                }
            }

            while let Some(message) = api.recv(&mut msg_buf) {
                match call.take(message) {
                    Got::Status(status) => {
                        loading = false;
                        http_code = 0;
                        if status != 0 {
                            body_len = 0;
                        }
                    }

                    Got::Record { index, record, last, .. } => {
                        if index == 0 {
                            if let Some(head) = HttpHead::decode(record) {
                                http_code = head.code;
                                expected_len = (head.length as usize).min(body.len());
                            }
                        } else {
                            let offset = (index as usize - 1) * 121;
                            if offset < body.len() {
                                let count = record.len().min(body.len() - offset);
                                body[offset..offset + count]
                                    .copy_from_slice(&record[..count]);
                            }
                        }

                        if last {
                            body_len = expected_len;
                            loading = false;
                        }
                    }

                    Got::Nothing => {}
                }
            }

            draw(
                api,
                &url[..url_len],
                selected,
                loading,
                http_code,
                &body,
                body_len,
            );

            let Some(event) = api.event(100) else {
                continue;
            };

            match event.kind {
                BUTTON => return 0,

                CROWN => {
                    let count = CHARS.len() as i32;
                    selected = (selected as i32 + event.delta as i32)
                        .rem_euclid(count) as usize;
                }

                TOUCH_DOWN => {
                    if event.y > 380 {
                        let address = core::str::from_utf8(&url[..url_len])
                            .unwrap_or("");

                        if http_get(1, address).is_some() {
                            body_len = 0;
                            expected_len = 0;
                            http_code = 0;
                            loading = true;
                            call.now(api.now());
                        }
                    } else if event.y > 300 && event.x > 300 {
                        if url_len > 0 {
                            url_len -= 1;
                        }
                    } else if event.y < 240 && url_len < url.len() {
                        url[url_len] = CHARS[selected];
                        url_len += 1;
                    }
                }

                PAUSE => {
                    loop {
                        if let Some(e) = api.event(u32::MAX) {
                            if e.kind == RESUME {
                                break;
                            }
                            if e.kind == BUTTON {
                                return 0;
                            }
                        }
                    }
                }

                RESUME => {}

                _ => {}
            }
        }
    }
                          }
