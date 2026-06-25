use crate::cli::PaperSize;

pub struct PaperConfig {
    pub width_mm: f64,
    pub height_mm: f64,
}

impl PaperConfig {
    pub fn from_paper_size(size: &PaperSize) -> Self {
        match size {
            PaperSize::A4 => Self { width_mm: 210.0, height_mm: 297.0 },
            PaperSize::Letter => Self { width_mm: 215.9, height_mm: 279.4 },
        }
    }
}

pub struct CardSlot {
    pub x_mm: f64,
    pub y_mm: f64,
}

pub struct LayoutConfig {
    pub paper: PaperConfig,
    pub card_width_mm: f64,
    pub card_height_mm: f64,
    pub cols: usize,
    pub rows: usize,
}

pub struct CutLine {
    pub x1_mm: f64,
    pub y1_mm: f64,
    pub x2_mm: f64,
    pub y2_mm: f64,
}

impl LayoutConfig {
    pub fn new(paper_size: &PaperSize) -> Self {
        Self {
            paper: PaperConfig::from_paper_size(paper_size),
            card_width_mm: 63.0,
            card_height_mm: 88.0,
            cols: 3,
            rows: 3,
        }
    }

    pub fn cards_per_page(&self) -> usize {
        self.cols * self.rows
    }

    pub fn pages_needed(&self, card_count: usize) -> usize {
        (card_count + self.cards_per_page() - 1) / self.cards_per_page()
    }

    fn content_width(&self) -> f64 {
        self.cols as f64 * self.card_width_mm
    }

    fn content_height(&self) -> f64 {
        self.rows as f64 * self.card_height_mm
    }

    fn margin_left(&self) -> f64 {
        (self.paper.width_mm - self.content_width()) / 2.0
    }

    fn margin_bottom(&self) -> f64 {
        (self.paper.height_mm - self.content_height()) / 2.0
    }

    pub fn card_positions(&self) -> Vec<CardSlot> {
        let mut slots = Vec::new();
        for row in 0..self.rows {
            for col in 0..self.cols {
                let x = self.margin_left() + col as f64 * self.card_width_mm;
                let y = self.margin_bottom() + (self.rows - 1 - row) as f64 * self.card_height_mm;
                slots.push(CardSlot { x_mm: x, y_mm: y });
            }
        }
        slots
    }

    pub fn cut_lines(&self) -> Vec<CutLine> {
        let mut lines = Vec::new();
        let ml = self.margin_left();
        let mb = self.margin_bottom();

        // vertical lines at card boundaries (including outer edges)
        for col in 0..=self.cols {
            let x = ml + col as f64 * self.card_width_mm;
            lines.push(CutLine {
                x1_mm: x, y1_mm: mb,
                x2_mm: x, y2_mm: mb + self.content_height(),
            });
        }

        // horizontal lines at card boundaries (including outer edges)
        for row in 0..=self.rows {
            let y = mb + row as f64 * self.card_height_mm;
            lines.push(CutLine {
                x1_mm: ml, y1_mm: y,
                x2_mm: ml + self.content_width(), y2_mm: y,
            });
        }

        lines
    }
}
