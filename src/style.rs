use tabled::{Table, settings::style::Style as TableStyle};

pub trait Style {
    fn style(&mut self) -> &mut Self;
}

impl Style for Table {
    fn style(&mut self) -> &mut Self {
        self.with(TableStyle::rounded())
    }
}
