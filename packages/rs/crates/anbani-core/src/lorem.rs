//! Fake-Georgian text (anbani `lorem`): char-rnn samples from Vepkhistqaosani for words, measured punctuation
//! frequencies, generated first and last names. Seedable; the sampler is a `SplitMix64`, so sequences are stable
//! across versions of this crate but not shared with the Python / JS ports (each has its own RNG).

use alloc::string::String;
use alloc::vec::Vec;

const FREQ_SUMMED: [(&str, f64); 4] =
    [("?", 0.006_977_481_76), ("!", 0.021_322_793_96), (".", 0.067_139_964_45), (",", 0.274_000_341_45)];

/// Words sampled from the epic.
pub static VEFXWORDS: [&str; 100] = [
    "გამიშვშეს",
    "მართალიპი",
    "შემომოცივე",
    "ვუხვა",
    "გავლსობა",
    "განდევანგაა",
    "მოასეხეს",
    "აქამდსაგებენ",
    "შვენინე",
    "უგანდეგო",
    "ქანულკულნი",
    "გემრუფენ",
    "ვეღათ",
    "მივისცა",
    "მუკამდის",
    "მივიმოწუქალსა",
    "მზლოს",
    "მოვლეხვე",
    "ვიმაქებდა",
    "ტატრესტინი",
    "მოითქოკა",
    "მდაგებენ",
    "ვიმცე",
    "სევა",
    "სათინესნი",
    "სადგმო",
    "გასჩვადეთ",
    "მესმანცა",
    "მივსცავე",
    "წეითო",
    "ძვილნი",
    "გამიზრიან",
    "დავათქვენო",
    "ზიცი",
    "მისცედი",
    "მონებანი",
    "დანამრისა",
    "სრცოტი",
    "ჰქონთავისთან",
    "მბრძენია",
    "წახსილთა",
    "მიხსენით",
    "სევნა",
    "გარდუკვრიდდა",
    "მიდამოდეს",
    "სიპყრო",
    "შემწოვლისა",
    "მამდიჰხლეან",
    "ბრჭნია",
    "დანენატანნი",
    "უკრთებოდემნი",
    "იციცოდა",
    "სამატა",
    "უფრნა",
    "გაეგარნების",
    "დავეჯარენა",
    "მტემურსა",
    "მშვენოდენო",
    "გიმძიმნო",
    "სამვზინო",
    "მოვლოცთა",
    "გიშვილა",
    "დაამჭებინ",
    "მითხროს",
    "მჟავალია",
    "მღურა",
    "ღინდეს",
    "მოაგხება",
    "გავიცეცხლი",
    "ტირსლით",
    "გავწყრეტა",
    "მყივნენ",
    "ქაფინს",
    "ბანგსა",
    "შემოხანა",
    "შემოვსჭვრეტით",
    "ამოვჰღერთო",
    "მოცადია",
    "მოეხვიდეს",
    "თქმია",
    "აღვია",
    "ვლამს",
    "მოხდიანები",
    "გორსვლითა",
    "გვთქვას",
    "გაუზადა",
    "დაბითქის",
    "ანვინა",
    "შრიდონი",
    "გავწიდნია",
    "ალვითანი",
    "მონიო",
    "სიტირენ",
    "გიშიხარნი",
    "დამიფემი",
    "მეგრეცა",
    "ფითჯი",
    "მსდევარსა",
    "განაზანდა",
    "საქმინ",
];

/// Generated first names.
pub static FNAMES: [&str; 100] = [
    "თავან",
    "ელიკონ",
    "კანტლიონ",
    "გველე",
    "ლადიან",
    "ლოტინე",
    "გიოლგვი",
    "გარსა",
    "თედა",
    "მეზნან",
    "მარტა",
    "აურაზგრ",
    "მადრა",
    "გიოგილი",
    "სონიკელიზანი",
    "ასსანო",
    "ნიგოლიტი",
    "ავაკვან",
    "მონს",
    "ნადრინ",
    "ნიფოლ",
    "ელიკორ",
    "დამას",
    "გიმრი",
    "ობარხი",
    "გამონ",
    "ონეს",
    "ზიხტინე",
    "იოკელ",
    "სატდა",
    "პარა",
    "იანეზ",
    "ზინა",
    "რანგი",
    "იანგორ",
    "აბსინე",
    "ლენტან",
    "ზისტიკობ",
    "ნიგოლო",
    "დატითნი",
    "მანთა",
    "არგიქ",
    "ზიკო",
    "ერლე",
    "იანე",
    "მევრენ",
    "ალტისა",
    "მარდია",
    "სისელ",
    "ხეისა",
    "ელუან",
    "დელო",
    "იეკლებ",
    "გუჩილი",
    "ტამა",
    "ნოკგოლ",
    "იაკისე",
    "დადო",
    "ართემრას",
    "ანაიკობე",
    "გტიტ",
    "თახსან",
    "ზანა",
    "ნუდო",
    "გიოდა",
    "ისსლიკონ",
    "ალსან",
    "რამოლე",
    "გუმან",
    "ნურმგინ",
    "თომითანი",
    "დევანე",
    "ელედეინ",
    "გიმოტ",
    "არარ",
    "ორმონ",
    "ნედრი",
    "დიმო",
    "ხოთიანეი",
    "გივა",
    "ნანტრე",
    "გიმოლ",
    "დოსონ",
    "იკანენ",
    "იკქორიზე",
    "გილტორგ",
    "დეტონ",
    "ერდეა",
    "ირეგლიზ",
    "რიდელუდან",
    "თოკან",
    "იანებენტიანა",
    "სიბო",
    "გიორტინ",
    "ნიკილან",
    "ელაკონ",
    "გიდრაბ",
    "ლეცთეტანტენდერიბ",
    "დამერ",
    "იეგო",
];

/// Generated last names.
pub static LNAMES: [&str; 100] = [
    "მახადენია",
    "ბსელრია",
    "ქავია",
    "კორეკია",
    "მარდია",
    "ბარდიონია",
    "გვიპიანდია",
    "ერთელია",
    "ნთხაია",
    "თედია",
    "ბოხაია",
    "ბარია",
    "ჭარტინია",
    "არპანია",
    "ჩურია",
    "მორცაწაზია",
    "არმელია",
    "მირბარია",
    "ჭველაქია",
    "ქარითია",
    "გართელია",
    "იანია",
    "დვანტონეიანია",
    "პუჩია",
    "დატილია",
    "კანია",
    "ვარდსვალია",
    "ყორთელია",
    "არდოლია",
    "რიკსქაშვილი",
    "გამექონთეშვილი",
    "ლუჩაშვილი",
    "მარდაშვილი",
    "პაბიშვილი",
    "არბერეშვილი",
    "ფოსურიშვილი",
    "ვაწოშვილი",
    "ჩრილიაშვილი",
    "ტიმაშვილი",
    "მავანიშვილი",
    "მადსიშვილი",
    "რედელიშვილი",
    "თეთაშვილი",
    "გარავაშვილი",
    "დარახაშვილი",
    "ბარჭამიშვილი",
    "ბარახაშვილი",
    "ანღელიშვილი",
    "მებაშვილი",
    "სურეჯიშვილი",
    "აკბთაკიშვილი",
    "ტუსონიშვილი",
    "ტოდერაშვილი",
    "ზაბხასიშვილი",
    "ქურაბერინიშვილი",
    "ებვეკაშვილი",
    "ერდითაშვილი",
    "ურბელიშვილი",
    "მენრმაშვილი",
    "ჯვილოძე",
    "ბადიაძე",
    "შვირიძე",
    "სასანაძე",
    "გიმაანიძე",
    "გვამავიძე",
    "არმემიძსძე",
    "ბარაძე",
    "აბეანიძე",
    "ოვონიძე",
    "ლამაშვიძე",
    "ობთახურაძე",
    "კერეძე",
    "ფვერიძე",
    "მაგიძე",
    "კადიაშიძე",
    "გამსანიძე",
    "მერტინიძე",
    "ბემეელიძე",
    "ტოხიძე",
    "ჭარატიძე",
    "ბადოსელი",
    "ვარისელი",
    "სირხელი",
    "სორხელი",
    "მარდელი",
    "გაშვითელი",
    "კოთაველი",
    "ოსთმინელი",
    "ერისელი",
    "ზთერტელი",
    "ხამიკელი",
    "მიბაძეშიელი",
    "იკთელი",
    "გთციტაბელი",
    "ასიცელი",
    "ობაიჭელი",
    "ლენთელი",
    "მემჩათერინონგბანინიძე",
    "ამედვაქსიკოგეიშვირი",
    "მორჟვონანდიშვილი",
];

/// A seeded generator with an optional custom word list.
#[derive(Debug, Clone)]
pub struct Lorem {
    state: u64,
    words: Option<Vec<String>>,
}

impl Default for Lorem {
    fn default() -> Self {
        Self::seeded(0x9E37_79B9_7F4A_7C15)
    }
}

impl Lorem {
    /// A generator seeded with `seed` (the same seed gives the same text).
    pub const fn seeded(seed: u64) -> Self {
        Self { state: seed, words: None }
    }

    /// Replace the word source used by `sentences` / `random_word`.
    pub fn load_wordlist(&mut self, words: Vec<String>) {
        self.words = Some(words);
    }

    #[expect(clippy::cast_precision_loss, reason = "53-bit mantissa draw: the top 53 bits are exact")]
    fn random(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Integer in `[lo, hi)`: `floor(random() * (hi - lo) + lo)`.
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "upstream floor(random() * (hi - lo) + lo) on small list lengths, non-negative by construction"
    )]
    fn random_number(&mut self, lo: usize, hi: usize) -> usize {
        (self.random() * (hi - lo) as f64 + lo as f64) as usize
    }

    fn random_mark(&mut self) -> &'static str {
        let r = self.random();
        for (mark, threshold) in FREQ_SUMMED {
            if r < threshold {
                return match mark {
                    "?" => "? ",
                    "!" => "! ",
                    "." => ". ",
                    _ => ", ",
                };
            }
        }
        " "
    }

    /// One word from the active list.
    pub fn random_word(&mut self) -> String {
        let custom = self.words.as_ref().map_or(0, Vec::len);
        match custom {
            n if n > 0 => {
                let i = self.random_number(0, n);
                self.words.as_ref().and_then(|w| w.get(i)).cloned().unwrap_or_default()
            }
            _ => {
                let i = self.random_number(0, VEFXWORDS.len());
                String::from(VEFXWORDS[i])
            }
        }
    }

    /// One first name.
    pub fn random_first_name(&mut self) -> &'static str {
        FNAMES[self.random_number(0, FNAMES.len())]
    }

    /// One last name.
    pub fn random_last_name(&mut self) -> &'static str {
        LNAMES[self.random_number(0, LNAMES.len())]
    }

    fn sentence(&mut self, nwords: usize) -> String {
        let mut s = String::new();
        for _ in 0..nwords {
            let w = self.random_word();
            s.push_str(&w);
            s.push_str(self.random_mark());
        }
        let mut out = String::from(s.trim_end_matches(crate::chars::is_space));
        if out.ends_with([',', '?', '!']) {
            out.pop();
        }
        if !out.is_empty() && !out.ends_with('.') {
            out.push('.');
        }
        out
    }

    /// One fake sentence of `nwords` words, period-terminated.
    pub fn sentences(&mut self, nwords: usize) -> String {
        self.sentence(nwords)
    }

    /// `npar` paragraphs of `nwords` words each, every paragraph followed by `newline`.
    pub fn paragraphs(&mut self, nwords: usize, npar: usize, newline: &str) -> String {
        let mut out = String::new();
        for _ in 0..npar {
            out.push_str(&self.sentence(nwords));
            out.push_str(newline);
        }
        out
    }

    /// `n` "First Last" names.
    pub fn names(&mut self, n: usize) -> Vec<String> {
        (0..n)
            .map(|_| {
                let mut s = String::from(self.random_first_name());
                s.push(' ');
                s.push_str(self.random_last_name());
                s
            })
            .collect()
    }
}
