use crate::history::History;
use core::fmt::Write;
use embedded_graphics::{
    prelude::*,
    primitives::{Line, PrimitiveStyle},
};
use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
    spi::SpiDevice,
};
use epd_waveshare::{
    epd1in54_v2::{Display1in54, Epd1in54},
    prelude::*,
};
use esp_println::println;
use heapless::String;
use u8g2_fonts::{
    fonts,
    types::{FontColor, HorizontalAlignment, VerticalPosition},
    FontRenderer,
};

pub struct Display<SPI, BUSY, DC, RST, DELAY> {
    epd: Epd1in54<SPI, BUSY, DC, RST, DELAY>,
    display: Display1in54,
    spi: SPI,
    delay: DELAY,
}

impl<SPI, BUSY, DC, RST, DELAY> Display<SPI, BUSY, DC, RST, DELAY>
where
    SPI: SpiDevice,
    DELAY: DelayNs,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
{
    pub fn new(
        mut spi: SPI,
        busy_pin: BUSY,
        data_command_pin: DC,
        reset_pin: RST,
        mut delay: DELAY,
    ) -> Self {
        let epd = Epd1in54::new(
            &mut spi,
            busy_pin,
            data_command_pin,
            reset_pin,
            &mut delay,
            Some(5),
        )
        .unwrap();

        let mut display = Display1in54::default();
        display.set_rotation(DisplayRotation::Rotate270);
        display.clear(Color::White).unwrap();

        Display {
            epd,
            display,
            spi,
            delay,
        }
    }

    pub fn draw(
        &mut self,
        history: &History,
        temperature: f32,
        battery_voltage: f32,
    ) -> Result<(), SPI::Error> {
        self.epd
            .set_lut(&mut self.spi, &mut self.delay, Some(RefreshLut::Full))?;

        self.draw_temperature(temperature);
        self.draw_voltage(battery_voltage);

        if history.len() > 0 {
            let latest_co2 = history.data_for_display().1.last().expect("History should not be empty");
            self.draw_co2(*latest_co2);
            self.draw_graph(history);
        }

        self.epd
            .update_and_display_frame(&mut self.spi, self.display.buffer(), &mut self.delay)?;

        self.epd.sleep(&mut self.spi, &mut self.delay)?;

        Ok(())
    }

    fn draw_co2(&mut self, co2: u16) {
        // TODO: Use the Results.
        let co2_font = FontRenderer::new::<fonts::u8g2_font_fub42_tr>();
        let mut co2_text = String::<32>::new();
        let _ = write!(&mut co2_text, "{co2}");
        co2_font
            .render_aligned(
                co2_text.as_str(),
                self.display.bounding_box().center() + Point::new(0, 5),
                VerticalPosition::Baseline,
                HorizontalAlignment::Center,
                FontColor::Transparent(Color::Black),
                &mut self.display,
            )
            .unwrap();

        let ppm_font = FontRenderer::new::<fonts::u8g2_font_fub25_tr>();
        let mut ppm_text = String::<32>::new();
        let _ = write!(&mut ppm_text, "ppm");
        ppm_font
            .render_aligned(
                ppm_text.as_str(),
                self.display.bounding_box().center() + Point::new(0, 35),
                VerticalPosition::Baseline,
                HorizontalAlignment::Center,
                FontColor::Transparent(Color::Black),
                &mut self.display,
            )
            .unwrap();
    }

    fn draw_temperature(&mut self, temperature: f32) {
        let temperature_font = FontRenderer::new::<fonts::u8g2_font_fub14_tr>();
        let mut temperature_text = String::<32>::new();
        let _ = write!(&mut temperature_text, "{temperature:.1} C");
        temperature_font
            .render_aligned(
                temperature_text.as_str(),
                Point::new(5, self.display.size().height as i32 - 5),
                VerticalPosition::Baseline,
                HorizontalAlignment::Left,
                FontColor::Transparent(Color::Black),
                &mut self.display,
            )
            .unwrap();
    }

    fn draw_voltage(&mut self, voltage: f32) {
        let voltage_font = FontRenderer::new::<fonts::u8g2_font_fub14_tr>();
        let mut voltage_text = String::<32>::new();
        let _ = write!(&mut voltage_text, "{voltage:.2} V");
        voltage_font
            .render_aligned(
                voltage_text.as_str(),
                Point::new(
                    self.display.size().width as i32 - 5,
                    self.display.size().height as i32 - 5,
                ),
                VerticalPosition::Baseline,
                HorizontalAlignment::Right,
                FontColor::Transparent(Color::Black),
                &mut self.display,
            )
            .unwrap();
    }

    fn draw_graph(&mut self, history: &History) {
        // Swapped because the display is rotated.
        let width = self.epd.height() as i32;
        let height = self.epd.width() as i32;

        let history_length = history.len();

        // Find max value.
        let max_co2 = history.max_value().expect("No history to display");

        for i in 0..(history_length - 1) {
            let x0 = ((i as i32) * width) / ((history_length - 1) as i32);
            let x1 = (((i + 1) as i32) * width) / ((history_length - 1) as i32);
            let y0 = height - ((history.at(i) as i32) * height) / (max_co2 as i32);
            let y1 = height - ((history.at(i + 1) as i32) * height) / (max_co2 as i32);
            let _ = Line::new(Point::new(x0, y0), Point::new(x1, y1))
                .into_styled(PrimitiveStyle::with_stroke(Color::Black, 2))
                .draw(&mut self.display);
        };
    }
}
