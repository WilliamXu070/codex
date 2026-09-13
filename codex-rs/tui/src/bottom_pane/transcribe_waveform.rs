pub(super) const TRANSCRIBE_WAVEFORM_WIDTH: usize = 5;

const TRANSCRIBE_BRAILLE_BLANK: u32 = 0x2800;
const TRANSCRIBE_BRAILLE_LEFT_DOTS: [u32; 4] = [0x02, 0x04, 0x01, 0x40];
const TRANSCRIBE_BRAILLE_RIGHT_DOTS: [u32; 4] = [0x10, 0x20, 0x08, 0x80];

fn transcribe_sample_level(sample: f32) -> usize {
    let sample = sample.clamp(/*min*/ 0.0, /*max*/ 1.0);
    if sample < 0.02 {
        0
    } else {
        (sample.sqrt() * TRANSCRIBE_BRAILLE_LEFT_DOTS.len() as f32)
            .ceil()
            .clamp(
                /*min*/ 1.0,
                /*max*/ TRANSCRIBE_BRAILLE_LEFT_DOTS.len() as f32,
            ) as usize
    }
}

fn transcribe_braille_column(sample: f32, dots: &[u32; 4]) -> u32 {
    dots.iter()
        .take(transcribe_sample_level(sample))
        .fold(/*init*/ 0, |cell, dot| cell | dot)
}

pub(super) fn transcribe_marker_text(samples: &[f32]) -> String {
    samples
        .chunks(/*chunk_size*/ 2)
        .take(TRANSCRIBE_WAVEFORM_WIDTH)
        .map(|chunk| {
            let left = chunk.first().copied().unwrap_or(/*default*/ 0.0);
            let right = chunk.get(/*index*/ 1).copied().unwrap_or(/*default*/ 0.0);
            let cell = TRANSCRIBE_BRAILLE_BLANK
                | transcribe_braille_column(left, &TRANSCRIBE_BRAILLE_LEFT_DOTS)
                | transcribe_braille_column(right, &TRANSCRIBE_BRAILLE_RIGHT_DOTS);
            char::from_u32(cell).unwrap_or('\u{2800}')
        })
        .collect()
}
