use keyboard_core::{Hsv, LightingState};

use crate::{ProtocolError, REPORT_ID, REPORT_SIZE};

const FAMILY: u8 = 0x06;
const LIGHTING_BODY_START: usize = 5;
const LIGHTING_BODY_LEN: usize = 11;

pub fn encode_lighting_read_request() -> [u8; REPORT_SIZE + 1] {
    command(0x0A)
}

pub fn encode_lighting_effect_request(effect: u8) -> [u8; REPORT_SIZE + 1] {
    let mut report = command(0x16);
    report[6] = 1;
    report[8] = effect;
    report
}

pub fn encode_lighting_write(state: LightingState) -> [u8; REPORT_SIZE + 1] {
    let mut report = command(0x0B);
    report[3] = LIGHTING_BODY_LEN as u8;
    report[6..17].copy_from_slice(&[
        state.kind,
        0,
        state.effect,
        state.brightness,
        state.speed,
        state.direction,
        if state.effect == 0 {
            0
        } else {
            u8::from(state.color_enabled)
        },
        state.single_color_index,
        state.hsv.hue,
        state.hsv.saturation,
        state.hsv.value,
    ]);
    report
}

pub fn parse_lighting_response(response: &[u8]) -> Result<LightingState, ProtocolError> {
    let body = lighting_body(response)?;
    // The 06 0A response is not laid out exactly like the 06 0B config body.
    // Physical reads use FF,H,S,V,pad for single colour, while dynamic reads use
    // 01,index,H,S,V. The 06 0B encoder above always uses color,index,H,S,V.
    let single_color = body[6] == 0xFF;
    let (single_color_index, hue, saturation, value) = if single_color {
        (0, body[7], body[8], body[9])
    } else {
        (body[7], body[8], body[9], body[10])
    };
    Ok(lighting_state(
        body,
        single_color,
        single_color_index,
        hue,
        saturation,
        value,
    ))
}

/// Decode the stable `color,index,H,S,V` body returned by `06 16` and `06 0B`.
pub fn parse_lighting_config_response(response: &[u8]) -> Result<LightingState, ProtocolError> {
    let body = lighting_body(response)?;
    Ok(lighting_state(
        body,
        body[6] != 0,
        body[7],
        body[8],
        body[9],
        body[10],
    ))
}

fn lighting_body(response: &[u8]) -> Result<&[u8], ProtocolError> {
    if response.len() < LIGHTING_BODY_START + LIGHTING_BODY_LEN {
        return Err(ProtocolError::ResponseTooShort {
            expected: LIGHTING_BODY_START + LIGHTING_BODY_LEN,
            actual: response.len(),
        });
    }
    if response[0] != 0xAA {
        return Err(ProtocolError::UnexpectedReplyPrefix(response[0]));
    }
    Ok(&response[LIGHTING_BODY_START..LIGHTING_BODY_START + LIGHTING_BODY_LEN])
}

fn lighting_state(
    body: &[u8],
    color_enabled: bool,
    single_color_index: u8,
    hue: u8,
    saturation: u8,
    value: u8,
) -> LightingState {
    LightingState {
        kind: body[0],
        effect: body[2],
        brightness: body[3],
        speed: body[4],
        direction: body[5],
        color_enabled,
        single_color_index,
        hsv: Hsv {
            hue,
            saturation,
            value,
        },
    }
}

fn command(command_byte: u8) -> [u8; REPORT_SIZE + 1] {
    let mut report = [0; REPORT_SIZE + 1];
    report[0] = REPORT_ID;
    report[1] = FAMILY;
    report[2] = command_byte;
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(effect: u8) -> LightingState {
        LightingState {
            kind: 1,
            effect,
            brightness: 4,
            speed: 3,
            direction: 2,
            color_enabled: true,
            single_color_index: 7,
            hsv: Hsv {
                hue: 128,
                saturation: 200,
                value: 255,
            },
        }
    }

    #[test]
    fn lighting_read_and_effect_requests_match_frontend() {
        assert_eq!(&encode_lighting_read_request()[1..3], &[0x06, 0x0A]);
        assert_eq!(
            &encode_lighting_effect_request(19)[1..9],
            &[0x06, 0x16, 0, 0, 0, 1, 0, 19]
        );
    }

    #[test]
    fn lighting_response_decodes_dynamic_fields() {
        let mut response = [0; REPORT_SIZE];
        response[0] = 0xAA;
        response[5..16].copy_from_slice(&[1, 0, 5, 4, 3, 2, 1, 7, 128, 200, 255]);
        let mut expected = state(5);
        expected.color_enabled = false;
        assert_eq!(parse_lighting_response(&response).unwrap(), expected);
    }

    #[test]
    fn lighting_response_decodes_single_colour_fields() {
        let mut response = [0; REPORT_SIZE];
        response[0] = 0xAA;
        response[5..16].copy_from_slice(&[1, 0, 5, 4, 3, 2, 0xFF, 128, 200, 255, 0]);
        let mut expected = state(5);
        expected.single_color_index = 0;
        assert_eq!(parse_lighting_response(&response).unwrap(), expected);
    }

    #[test]
    fn lighting_config_response_keeps_the_stable_tail_layout() {
        let mut response = [0; REPORT_SIZE];
        response[0] = 0xAA;
        response[5..16].copy_from_slice(&[1, 0, 20, 4, 0, 0, 1, 3, 10, 20, 30]);
        let parsed = parse_lighting_config_response(&response).unwrap();
        assert_eq!(parsed.effect, 20);
        assert!(parsed.color_enabled);
        assert_eq!(parsed.single_color_index, 3);
        assert_eq!(
            parsed.hsv,
            Hsv {
                hue: 10,
                saturation: 20,
                value: 30
            }
        );
    }

    #[test]
    fn lighting_write_has_exact_layout_and_forces_off_color_state() {
        let on = encode_lighting_write(state(5));
        assert_eq!(
            &on[1..17],
            &[0x06, 0x0B, 11, 0, 0, 1, 0, 5, 4, 3, 2, 1, 7, 128, 200, 255]
        );

        let off = encode_lighting_write(state(0));
        assert_eq!(off[12], 0);
    }
}
