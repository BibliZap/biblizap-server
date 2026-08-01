#[derive(Clone, PartialEq, Debug)]
pub(super) struct DateScale {
    start_date: time::Date,
    end_date: time::Date,
}

impl DateScale {
    pub(super) fn new(start_date: time::Date, end_date: time::Date) -> Self {
        Self {
            start_date,
            end_date,
        }
    }

    pub(super) fn date_to_chart_x(&self, date: time::Date) -> f64 {
        (date - self.start_date).whole_days() as f64
    }

    pub(super) fn chart_width_in_chart_units(&self) -> f64 {
        (self.end_date - self.start_date).whole_days() as f64
    }

    fn tick_frequency_from_range(&self) -> time::Duration {
        let total_days = (self.end_date - self.start_date).whole_days();

        if total_days <= 14 {
            time::Duration::days(2)
        } else if total_days <= 90 {
            time::Duration::weeks(1)
        } else if total_days <= 365 {
            time::Duration::days(90)
        } else if total_days <= 730 {
            time::Duration::days(180)
        } else {
            time::Duration::days(365)
        }
    }

    pub(super) fn make_ticks(&self) -> Vec<time::Date> {
        let mut ticks = Vec::new();
        let mut current_date = self.start_date;
        let tick_frequency = self.tick_frequency_from_range();

        while current_date <= self.end_date {
            ticks.push(current_date);
            current_date = current_date + tick_frequency;
        }

        ticks
    }
}

#[derive(Clone, PartialEq, Debug)]
pub(super) struct CountScale {
    min_count: i64,
    max_count: i64,
    n_ticks: i32,
}

impl CountScale {
    pub(super) fn new(min_count: i64, max_count: i64, n_ticks: i32) -> Self {
        Self {
            min_count,
            max_count,
            n_ticks,
        }
    }

    pub(super) fn count_to_chart_y(&self, count: i64) -> f64 {
        (count - self.min_count) as f64
    }

    pub(super) fn chart_height_in_chart_units(&self) -> f64 {
        (self.max_count - self.min_count) as f64
    }

    fn tick_index_to_percent(&self, index: i32) -> f64 {
        index as f64 / self.n_ticks as f64
    }

    fn tick_chart_y_to_label(&self, chart_y: f64) -> String {
        let count_value = self.min_count + chart_y as i64;
        count_value.to_string()
    }

    fn tick_index_to_chart_y(&self, index: i32) -> f64 {
        self.chart_height_in_chart_units() * self.tick_index_to_percent(index)
    }

    pub(super) fn make_ticks_and_labels(&self) -> Vec<(f64, String)> {
        let mut ticks = Vec::new();
        for i in 0..=self.n_ticks {
            let y_position = self.tick_index_to_chart_y(i);
            let count_label = self.tick_chart_y_to_label(y_position);
            ticks.push((y_position, count_label));
        }
        ticks
    }
}

#[derive(Clone, PartialEq, Debug)]
pub(super) struct ChartScale {
    pub(super) x: DateScale,
    pub(super) y: CountScale,
}

impl ChartScale {
    pub(super) fn new(x: DateScale, y: CountScale) -> Self {
        Self { x, y }
    }
}
