//! Geçmiş: grafikler için sabit kapasiteli halka tamponlar (§5).

use std::collections::VecDeque;

use crate::model::{Metric, Snapshot};

/// Sabit kapasiteli halka tampon; dolunca en eski öğenin üstüne yazılır.
#[derive(Debug, Clone)]
pub struct Ring<T> {
    buf: VecDeque<T>,
    cap: usize,
}

impl<T> Ring<T> {
    /// `cap` kapasiteli boş tampon (en az 1 kabul edilir).
    pub fn new(cap: usize) -> Self {
        let cap = cap.max(1);
        Self {
            buf: VecDeque::with_capacity(cap),
            cap,
        }
    }

    /// Yeni öğe ekler; tampon doluysa en eskisi atılır.
    pub fn push(&mut self, v: T) {
        if self.buf.len() == self.cap {
            self.buf.pop_front();
        }
        self.buf.push_back(v);
    }

    /// Öğe sayısı.
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Tampon boş mu?
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Eskiden yeniye doğru yineleyici.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> + ExactSizeIterator {
        self.buf.iter()
    }

    /// En yeni öğe.
    pub fn last(&self) -> Option<&T> {
        self.buf.back()
    }
}

/// Metrik başına geçmiş serileri. `None` = okunamadı (grafikte boşluk).
#[derive(Debug, Clone)]
pub struct History {
    freq_act: Ring<Option<f64>>,
    power_w: Ring<Option<f64>>,
    temp_c: Ring<Option<f64>>,
    busy_total: Ring<Option<f64>>,
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

impl History {
    /// Her serinin kapasitesi (300 nokta; 1 sn aralıkla 5 dk).
    pub const CAPACITY: usize = 300;

    /// Boş geçmiş.
    pub fn new() -> Self {
        Self {
            freq_act: Ring::new(Self::CAPACITY),
            power_w: Ring::new(Self::CAPACITY),
            temp_c: Ring::new(Self::CAPACITY),
            busy_total: Ring::new(Self::CAPACITY),
        }
    }

    /// Snapshot'tan her seriye bir nokta ekler. `busy_total` = en yüksek motor doluluğu.
    pub fn push(&mut self, s: &Snapshot) {
        fn opt<T: Copy + Into<f64>>(m: &Metric<T>) -> Option<f64> {
            m.value().map(|v| (*v).into())
        }
        self.freq_act.push(opt(&s.freq.act));
        self.power_w.push(opt(&s.power_w));
        self.temp_c.push(opt(&s.temp_c));
        let busy = s
            .engines
            .iter()
            .map(|e| f64::from(e.busy_pct))
            .fold(None, |acc: Option<f64>, b| {
                Some(acc.map_or(b, |a| a.max(b)))
            });
        self.busy_total.push(busy);
    }

    /// Gerçekleşen frekans serisi (MHz).
    pub fn freq_act(&self) -> &Ring<Option<f64>> {
        &self.freq_act
    }

    /// Güç serisi (W).
    pub fn power_w(&self) -> &Ring<Option<f64>> {
        &self.power_w
    }

    /// Sıcaklık serisi (°C).
    pub fn temp_c(&self) -> &Ring<Option<f64>> {
        &self.temp_c
    }

    /// Toplam doluluk serisi (%; en yüksek motor).
    pub fn busy_total(&self) -> &Ring<Option<f64>> {
        &self.busy_total
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{EngineClass, EngineUsage};
    use std::time::Instant;

    #[test]
    fn ring_overflow_keeps_last_three() {
        let mut r = Ring::new(3);
        assert!(r.is_empty());
        for i in 1..=5 {
            r.push(i);
        }
        assert_eq!(r.len(), 3);
        assert_eq!(r.iter().copied().collect::<Vec<_>>(), vec![3, 4, 5]);
        assert_eq!(r.last(), Some(&5));
    }

    #[test]
    fn history_maps_values_and_gaps() {
        let mut h = History::new();
        let mut s = Snapshot::empty(Instant::now());
        h.push(&s); // hepsi Unavailable
        s.freq.act = Metric::Value(1200);
        s.power_w = Metric::Value(7.5);
        s.engines = vec![
            EngineUsage {
                class: EngineClass::Render,
                busy_pct: 10.0,
            },
            EngineUsage {
                class: EngineClass::Video,
                busy_pct: 40.0,
            },
        ];
        h.push(&s);
        assert_eq!(
            h.freq_act().iter().copied().collect::<Vec<_>>(),
            vec![None, Some(1200.0)]
        );
        assert_eq!(h.power_w().last(), Some(&Some(7.5)));
        assert_eq!(h.temp_c().last(), Some(&None));
        assert_eq!(
            h.busy_total().iter().copied().collect::<Vec<_>>(),
            vec![None, Some(40.0)]
        );
    }
}
