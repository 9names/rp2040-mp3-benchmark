#![allow(clippy::empty_loop)]
//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

use bsp::entry;
use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::v2::OutputPin;
use panic_probe as _;

use rp_pico as bsp;

use bsp::hal::{clocks::init_clocks_and_plls, pac, sio::Sio, watchdog::Watchdog};

use adafruit_mp3_sys::ffi::*;

static MP3: &[u8] = include_bytes!("../test.mp3");

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let _core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let _clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let timer = bsp::hal::Timer::new(pac.TIMER, &mut pac.RESETS);

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut led_pin = pins.led.into_push_pull_output();

    led_pin.set_high().unwrap();
    let start_time = timer.get_counter_low();

    info!("picomp3lib decoding start");
    type Mp3ptrT = *const u8;
    type Mp3ptrptrT = *mut Mp3ptrT;
    let mut mp3ptr: Mp3ptrT = MP3.as_ptr();
    let mp3ptrptr: Mp3ptrptrT = &mut mp3ptr;
    info!(
        "mp3ptr {:?}, mp3ptrptr {:?}, mp3ptrptr_pointee {:?}",
        mp3ptr,
        mp3ptrptr,
        unsafe { *mp3ptrptr }
    );
    let mut bytes_left = MP3.len() as i32;
    let mp3dec = unsafe { adafruit_mp3_sys::ffi::MP3InitDecoder() };
    let start = unsafe { adafruit_mp3_sys::ffi::MP3FindSyncWord(mp3ptr, bytes_left) };
    bytes_left -= start;
    info!("start: {}", start);

    // Update our MP3 pointer to skip past the id3 tags
    let mut mp3ptr: Mp3ptrT = MP3.as_ptr().wrapping_add(start.try_into().unwrap());
    let mp3ptrptr: Mp3ptrptrT = &mut mp3ptr;

    let mut frame: _MP3FrameInfo = _MP3FrameInfo {
        bitrate: 0,
        nChans: 0,
        samprate: 0,
        bitsPerSample: 0,
        outputSamps: 0,
        layer: 0,
        version: 0,
    };

    info!(
        "mp3ptr {:?}, mp3ptrptr {:?}, mp3ptrptr_pointee {:?}",
        mp3ptr,
        mp3ptrptr,
        unsafe { *mp3ptrptr }
    );
    let f = unsafe { MP3GetNextFrameInfo(mp3dec, &mut frame, mp3ptr) };
    info!("MP3GetNextFrameInfo response: {:?}", f);
    info!(
        "info: {} {} {} {} {} {} {}",
        frame.bitrate,
        frame.nChans,
        frame.samprate,
        frame.bitsPerSample,
        frame.outputSamps,
        frame.layer,
        frame.version,
    );

    let decode_len = (frame.bitsPerSample >> 3) * frame.outputSamps;
    info!("decoded_len = {}", decode_len);
    let mut newlen = bytes_left;
    let mut buf = [0i16; 4608 / 2];

    while newlen > 0 {
        let decoded = unsafe { MP3Decode(mp3dec, mp3ptrptr, &mut newlen, buf.as_mut_ptr(), 0) };
        if decoded != 0 {
            let decoded = match decoded {
                0 => "Okay",
                -1 => "ERR_MP3_INDATA_UNDERFLOW",
                -2 => "ERR_MP3_MAINDATA_UNDERFLOW",
                -3 => "ERR_MP3_FREE_BITRATE_SYNC",
                -4 => "ERR_MP3_OUT_OF_MEMORY",
                -5 => "ERR_MP3_NULL_POINTER",
                -6 => "ERR_MP3_INVALID_FRAMEHEADER",
                -7 => "ERR_MP3_INVALID_SIDEINFO",
                -8 => "ERR_MP3_INVALID_SCALEFACT",
                -9 => "ERR_MP3_INVALID_HUFFCODES",
                -10 => "ERR_MP3_INVALID_DEQUANTIZE",
                -11 => "ERR_MP3_INVALID_IMDCT",
                -12 => "ERR_MP3_INVALID_SUBBAND",
                -9999 => "ERR_UNKNOWN",
                _ => "ERR_INVALID_ERROR",
            };
            info!("Decoded {}", decoded);
        }
        unsafe { MP3GetLastFrameInfo(mp3dec, &mut frame) };
        for _i in &buf[0..frame.outputSamps as usize] {
            // todo: use these samples
        }
    }
    let end_time = timer.get_counter_low();
    let elapsed = (end_time - start_time) as f64 / 1_000_000f64;
    info!(
        "decoding took {} seconds which is {}% of realtime",
        elapsed,
        (11f64 / elapsed) * 100f64
    );
    info!("done");
    loop {}
}

// End of file
