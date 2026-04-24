use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct Board {
    // Minimal representation: 0..63, piece chars like 'P','p','N' etc or '.'
    squares: [u8; 64],
    side_to_move: u8, // b'w' or b'b'
    castle: u8,       // bits: 1=K 2=Q 4=k 8=q
    ep: i8,           // -1 none else 0..63
}

impl Board {
    pub fn new() -> Self {
        Self {
            squares: [b'.'; 64],
            side_to_move: b'w',
            castle: 0b1111,
            ep: -1,
        }
    }

    pub fn load_fen(&mut self, fen: &str) -> Result<(), String> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.is_empty() {
            return Err("Empty FEN".into());
        }
        let board = parts[0];
        self.squares = [b'.'; 64];
        let mut rank = 7i32;
        let mut file = 0i32;
        for ch in board.chars() {
            if ch == '/' {
                rank -= 1;
                file = 0;
                continue;
            }
            if ch.is_ascii_digit() {
                file += ch.to_digit(10).unwrap() as i32;
                continue;
            }
            if file > 7 || rank < 0 {
                return Err("Invalid FEN board".into());
            }
            let idx = (rank * 8 + file) as usize;
            self.squares[idx] = ch as u8;
            file += 1;
        }
        if parts.len() >= 2 {
            self.side_to_move = if parts[1] == "b" { b'b' } else { b'w' };
        }
        self.castle = 0;
        if parts.len() >= 3 {
            let c = parts[2];
            if c.contains('K') {
                self.castle |= 1;
            }
            if c.contains('Q') {
                self.castle |= 2;
            }
            if c.contains('k') {
                self.castle |= 4;
            }
            if c.contains('q') {
                self.castle |= 8;
            }
        }
        self.ep = -1;
        if parts.len() >= 4 && parts[3] != "-" {
            if let Some(sq) = alg_to_sq(parts[3]) {
                self.ep = sq as i8;
            }
        }
        Ok(())
    }

    pub fn legal_moves_uci(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut pseudo = Vec::new();
        self.gen_pseudo(&mut pseudo);
        let us_white = self.side_to_move == b'w';
        for mv in pseudo {
            let mut b = self.clone();
            b.apply_unchecked(&mv);
            if !b.in_check(us_white) {
                out.push(mv.to_uci());
            }
        }
        out
    }

    fn gen_pseudo(&self, out: &mut Vec<Move>) {
        let us_white = self.side_to_move == b'w';
        for from in 0..64usize {
            let p = self.squares[from];
            if p == b'.' {
                continue;
            }
            let is_white = p.is_ascii_uppercase();
            if is_white != us_white {
                continue;
            }
            match p.to_ascii_lowercase() {
                b'p' => self.gen_pawn(from, is_white, out),
                b'n' => self.gen_knight(from, is_white, out),
                b'b' => self.gen_slider(from, is_white, out, &[(1, 1), (1, -1), (-1, 1), (-1, -1)]),
                b'r' => self.gen_slider(from, is_white, out, &[(1, 0), (-1, 0), (0, 1), (0, -1)]),
                b'q' => self.gen_slider(
                    from,
                    is_white,
                    out,
                    &[
                        (1, 1),
                        (1, -1),
                        (-1, 1),
                        (-1, -1),
                        (1, 0),
                        (-1, 0),
                        (0, 1),
                        (0, -1),
                    ],
                ),
                b'k' => self.gen_king(from, is_white, out),
                _ => {}
            }
        }
    }

    fn gen_pawn(&self, from: usize, white: bool, out: &mut Vec<Move>) {
        let r = (from / 8) as i32;
        let f = (from % 8) as i32;
        let dir = if white { 1 } else { -1 };
        let start_rank = if white { 1 } else { 6 };
        let promo_rank = if white { 6 } else { 1 };

        let one_r = r + dir;
        if one_r >= 0 && one_r <= 7 {
            let to = (one_r * 8 + f) as usize;
            if self.squares[to] == b'.' {
                if r == promo_rank {
                    for prom in [b'q', b'r', b'b', b'n'] {
                        out.push(Move::new(from, to).with_promo(prom));
                    }
                } else {
                    out.push(Move::new(from, to));
                    if r == start_rank {
                        let two_r = r + dir * 2;
                        let to2 = (two_r * 8 + f) as usize;
                        if self.squares[to2] == b'.' {
                            out.push(Move::new(from, to2).with_double_push());
                        }
                    }
                }
            }
        }

        for df in [-1, 1] {
            let nf = f + df;
            let nr = r + dir;
            if nf < 0 || nf > 7 || nr < 0 || nr > 7 {
                continue;
            }
            let to = (nr * 8 + nf) as usize;
            let dst = self.squares[to];
            if dst != b'.' && dst.is_ascii_uppercase() != white {
                if r == promo_rank {
                    for prom in [b'q', b'r', b'b', b'n'] {
                        out.push(Move::new(from, to).with_promo(prom));
                    }
                } else {
                    out.push(Move::new(from, to));
                }
            }

            // en passant
            if self.ep >= 0 && self.ep as usize == to {
                out.push(Move::new(from, to).with_en_passant());
            }
        }
    }

    fn gen_knight(&self, from: usize, white: bool, out: &mut Vec<Move>) {
        let r = (from / 8) as i32;
        let f = (from % 8) as i32;
        for (dr, df) in [
            (2, 1),
            (2, -1),
            (-2, 1),
            (-2, -1),
            (1, 2),
            (1, -2),
            (-1, 2),
            (-1, -2),
        ] {
            let nr = r + dr;
            let nf = f + df;
            if nr < 0 || nr > 7 || nf < 0 || nf > 7 {
                continue;
            }
            let to = (nr * 8 + nf) as usize;
            let dst = self.squares[to];
            if dst == b'.' || dst.is_ascii_uppercase() != white {
                out.push(Move::new(from, to));
            }
        }
    }

    fn gen_slider(&self, from: usize, white: bool, out: &mut Vec<Move>, dirs: &[(i32, i32)]) {
        let r0 = (from / 8) as i32;
        let f0 = (from % 8) as i32;
        for (dr, df) in dirs {
            let mut r = r0 + *dr;
            let mut f = f0 + *df;
            while r >= 0 && r <= 7 && f >= 0 && f <= 7 {
                let to = (r * 8 + f) as usize;
                let dst = self.squares[to];
                if dst == b'.' {
                    out.push(Move::new(from, to));
                } else {
                    if dst.is_ascii_uppercase() != white {
                        out.push(Move::new(from, to));
                    }
                    break;
                }
                r += *dr;
                f += *df;
            }
        }
    }

    fn gen_king(&self, from: usize, white: bool, out: &mut Vec<Move>) {
        let r = (from / 8) as i32;
        let f = (from % 8) as i32;
        for dr in -1..=1 {
            for df in -1..=1 {
                if dr == 0 && df == 0 {
                    continue;
                }
                let nr = r + dr;
                let nf = f + df;
                if nr < 0 || nr > 7 || nf < 0 || nf > 7 {
                    continue;
                }
                let to = (nr * 8 + nf) as usize;
                let dst = self.squares[to];
                if dst == b'.' || dst.is_ascii_uppercase() != white {
                    out.push(Move::new(from, to));
                }
            }
        }

        // castling
        if white && from == alg_to_sq("e1").unwrap() {
            if (self.castle & 1) != 0 && self.squares[alg_to_sq("f1").unwrap()] == b'.' && self.squares[alg_to_sq("g1").unwrap()] == b'.' {
                if !self.in_check(true)
                    && !self.is_attacked(alg_to_sq("f1").unwrap(), false)
                    && !self.is_attacked(alg_to_sq("g1").unwrap(), false)
                {
                    out.push(Move::new(from, alg_to_sq("g1").unwrap()).with_castle());
                }
            }
            if (self.castle & 2) != 0
                && self.squares[alg_to_sq("d1").unwrap()] == b'.'
                && self.squares[alg_to_sq("c1").unwrap()] == b'.'
                && self.squares[alg_to_sq("b1").unwrap()] == b'.'
            {
                if !self.in_check(true)
                    && !self.is_attacked(alg_to_sq("d1").unwrap(), false)
                    && !self.is_attacked(alg_to_sq("c1").unwrap(), false)
                {
                    out.push(Move::new(from, alg_to_sq("c1").unwrap()).with_castle());
                }
            }
        }
        if !white && from == alg_to_sq("e8").unwrap() {
            if (self.castle & 4) != 0 && self.squares[alg_to_sq("f8").unwrap()] == b'.' && self.squares[alg_to_sq("g8").unwrap()] == b'.' {
                if !self.in_check(false)
                    && !self.is_attacked(alg_to_sq("f8").unwrap(), true)
                    && !self.is_attacked(alg_to_sq("g8").unwrap(), true)
                {
                    out.push(Move::new(from, alg_to_sq("g8").unwrap()).with_castle());
                }
            }
            if (self.castle & 8) != 0
                && self.squares[alg_to_sq("d8").unwrap()] == b'.'
                && self.squares[alg_to_sq("c8").unwrap()] == b'.'
                && self.squares[alg_to_sq("b8").unwrap()] == b'.'
            {
                if !self.in_check(false)
                    && !self.is_attacked(alg_to_sq("d8").unwrap(), true)
                    && !self.is_attacked(alg_to_sq("c8").unwrap(), true)
                {
                    out.push(Move::new(from, alg_to_sq("c8").unwrap()).with_castle());
                }
            }
        }
    }

    fn king_sq(&self, white: bool) -> Option<usize> {
        let k = if white { b'K' } else { b'k' };
        self.squares.iter().position(|&p| p == k)
    }

    fn in_check(&self, white: bool) -> bool {
        let Some(ksq) = self.king_sq(white) else { return false; };
        self.is_attacked(ksq, !white)
    }

    fn is_attacked(&self, sq: usize, by_white: bool) -> bool {
        let r = (sq / 8) as i32;
        let f = (sq % 8) as i32;

        // pawn attacks
        let pdir = if by_white { 1 } else { -1 };
        for df in [-1, 1] {
            let nr = r - pdir; // from pawn perspective
            let nf = f + df;
            if nr >= 0 && nr <= 7 && nf >= 0 && nf <= 7 {
                let from = (nr * 8 + nf) as usize;
                let p = self.squares[from];
                if p != b'.' && p.is_ascii_uppercase() == by_white && p.to_ascii_lowercase() == b'p' {
                    return true;
                }
            }
        }

        // knights
        for (dr, df) in [
            (2, 1),
            (2, -1),
            (-2, 1),
            (-2, -1),
            (1, 2),
            (1, -2),
            (-1, 2),
            (-1, -2),
        ] {
            let nr = r + dr;
            let nf = f + df;
            if nr < 0 || nr > 7 || nf < 0 || nf > 7 {
                continue;
            }
            let from = (nr * 8 + nf) as usize;
            let p = self.squares[from];
            if p != b'.' && p.is_ascii_uppercase() == by_white && p.to_ascii_lowercase() == b'n' {
                return true;
            }
        }

        // sliders: bishop/rook/queen
        for (dr, df, kind) in [
            (1, 1, b'b'),
            (1, -1, b'b'),
            (-1, 1, b'b'),
            (-1, -1, b'b'),
            (1, 0, b'r'),
            (-1, 0, b'r'),
            (0, 1, b'r'),
            (0, -1, b'r'),
        ] {
            let mut nr = r + dr;
            let mut nf = f + df;
            while nr >= 0 && nr <= 7 && nf >= 0 && nf <= 7 {
                let from = (nr * 8 + nf) as usize;
                let p = self.squares[from];
                if p != b'.' {
                    if p.is_ascii_uppercase() == by_white {
                        let pl = p.to_ascii_lowercase();
                        if pl == kind || pl == b'q' {
                            return true;
                        }
                    }
                    break;
                }
                nr += dr;
                nf += df;
            }
        }

        // king adjacency
        for dr in -1..=1 {
            for df in -1..=1 {
                if dr == 0 && df == 0 {
                    continue;
                }
                let nr = r + dr;
                let nf = f + df;
                if nr < 0 || nr > 7 || nf < 0 || nf > 7 {
                    continue;
                }
                let from = (nr * 8 + nf) as usize;
                let p = self.squares[from];
                if p != b'.' && p.is_ascii_uppercase() == by_white && p.to_ascii_lowercase() == b'k' {
                    return true;
                }
            }
        }
        false
    }

    fn apply_unchecked(&mut self, mv: &Move) {
        self.ep = -1;
        let piece = self.squares[mv.from];
        let white = piece.is_ascii_uppercase();

        // castling rook moves
        if mv.castle {
            if white {
                if mv.to == alg_to_sq("g1").unwrap() {
                    // h1->f1
                    let rf = alg_to_sq("h1").unwrap();
                    let rt = alg_to_sq("f1").unwrap();
                    self.squares[rt] = self.squares[rf];
                    self.squares[rf] = b'.';
                } else if mv.to == alg_to_sq("c1").unwrap() {
                    let rf = alg_to_sq("a1").unwrap();
                    let rt = alg_to_sq("d1").unwrap();
                    self.squares[rt] = self.squares[rf];
                    self.squares[rf] = b'.';
                }
                self.castle &= !0b0011;
            } else {
                if mv.to == alg_to_sq("g8").unwrap() {
                    let rf = alg_to_sq("h8").unwrap();
                    let rt = alg_to_sq("f8").unwrap();
                    self.squares[rt] = self.squares[rf];
                    self.squares[rf] = b'.';
                } else if mv.to == alg_to_sq("c8").unwrap() {
                    let rf = alg_to_sq("a8").unwrap();
                    let rt = alg_to_sq("d8").unwrap();
                    self.squares[rt] = self.squares[rf];
                    self.squares[rf] = b'.';
                }
                self.castle &= !0b1100;
            }
        }

        // en passant capture
        if mv.en_passant {
            let to_rank = mv.to / 8;
            let cap_sq = if white {
                (to_rank - 1) * 8 + (mv.to % 8)
            } else {
                (to_rank + 1) * 8 + (mv.to % 8)
            };
            self.squares[cap_sq] = b'.';
        }

        self.squares[mv.from] = b'.';
        self.squares[mv.to] = if mv.promo != 0 {
            if white { mv.promo.to_ascii_uppercase() } else { mv.promo }
        } else {
            piece
        };

        // set ep square on double pawn push
        if mv.double_push && piece.to_ascii_lowercase() == b'p' {
            let from_r = mv.from / 8;
            let to_r = mv.to / 8;
            let mid_r = (from_r + to_r) / 2;
            self.ep = (mid_r * 8 + (mv.from % 8)) as i8;
        }

        // update castling rights if king/rook moved
        match piece {
            b'K' => self.castle &= !0b0011,
            b'k' => self.castle &= !0b1100,
            b'R' => {
                if mv.from == alg_to_sq("h1").unwrap() { self.castle &= !1; }
                if mv.from == alg_to_sq("a1").unwrap() { self.castle &= !2; }
            }
            b'r' => {
                if mv.from == alg_to_sq("h8").unwrap() { self.castle &= !4; }
                if mv.from == alg_to_sq("a8").unwrap() { self.castle &= !8; }
            }
            _ => {}
        }

        self.side_to_move = if self.side_to_move == b'w' { b'b' } else { b'w' };
    }
}

fn sq_to_alg(sq: usize) -> String {
    let file = (sq % 8) as u8;
    let rank = (sq / 8) as u8;
    let f = (b'a' + file) as char;
    let r = (b'1' + rank) as char;
    format!("{f}{r}")
}

static BOARD_STORE: Mutex<Vec<Board>> = Mutex::new(Vec::new());

pub fn board_new() -> i64 {
    let mut store = BOARD_STORE.lock().expect("board store poisoned");
    store.push(Board::new());
    (store.len() - 1) as i64
}

pub fn board_load(board_id: i64, fen: &str) -> Result<(), String> {
    let fen = if fen == "startpos" {
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
    } else {
        fen
    };
    let mut store = BOARD_STORE.lock().map_err(|_| "board store poisoned".to_string())?;
    let b = store
        .get_mut(board_id as usize)
        .ok_or_else(|| "board id out of range".to_string())?;
    b.load_fen(fen)
}

pub fn board_legalmoves(board_id: i64) -> Result<Vec<String>, String> {
    let store = BOARD_STORE.lock().map_err(|_| "board store poisoned".to_string())?;
    let b = store
        .get(board_id as usize)
        .ok_or_else(|| "board id out of range".to_string())?;
    Ok(b.legal_moves_uci())
}

pub fn board_piece_at(board_id: i64, sq: usize) -> Result<u8, String> {
    if sq >= 64 {
        return Err("square out of range".into());
    }
    let store = BOARD_STORE.lock().map_err(|_| "board store poisoned".to_string())?;
    let b = store
        .get(board_id as usize)
        .ok_or_else(|| "board id out of range".to_string())?;
    Ok(b.squares[sq])
}

pub fn board_apply_uci(board_id: i64, mv: &str) -> Result<(), String> {
    let mut store = BOARD_STORE.lock().map_err(|_| "board store poisoned".to_string())?;
    let b = store
        .get_mut(board_id as usize)
        .ok_or_else(|| "board id out of range".to_string())?;
    let legal = b.legal_moves_uci();
    if !legal.iter().any(|m| m == mv) {
        return Err("illegal move".into());
    }
    let mv = Move::from_uci(mv)?;
    b.apply_unchecked(&mv);
    Ok(())
}

fn alg_to_sq(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.len() != 2 {
        return None;
    }
    let f = bytes[0];
    let r = bytes[1];
    if !(b'a'..=b'h').contains(&f) || !(b'1'..=b'8').contains(&r) {
        return None;
    }
    let file = (f - b'a') as usize;
    let rank = (r - b'1') as usize;
    Some(rank * 8 + file)
}

#[derive(Debug, Clone, Copy)]
struct Move {
    from: usize,
    to: usize,
    promo: u8, // lowercase piece char (qrbn) or 0
    en_passant: bool,
    castle: bool,
    double_push: bool,
}

impl Move {
    fn new(from: usize, to: usize) -> Self {
        Self {
            from,
            to,
            promo: 0,
            en_passant: false,
            castle: false,
            double_push: false,
        }
    }
    fn with_promo(mut self, p: u8) -> Self {
        self.promo = p;
        self
    }
    fn with_en_passant(mut self) -> Self {
        self.en_passant = true;
        self
    }
    fn with_castle(mut self) -> Self {
        self.castle = true;
        self
    }
    fn with_double_push(mut self) -> Self {
        self.double_push = true;
        self
    }
    fn to_uci(&self) -> String {
        let mut s = format!("{}{}", sq_to_alg(self.from), sq_to_alg(self.to));
        if self.promo != 0 {
            s.push(self.promo as char);
        }
        s
    }
    fn from_uci(mv: &str) -> Result<Self, String> {
        let m = mv.as_bytes();
        if m.len() < 4 {
            return Err("move must be like e2e4".into());
        }
        let from = alg_to_sq(&mv[0..2]).ok_or_else(|| "bad from-square".to_string())?;
        let to = alg_to_sq(&mv[2..4]).ok_or_else(|| "bad to-square".to_string())?;
        let mut out = Move::new(from, to);
        if m.len() >= 5 {
            let p = m[4].to_ascii_lowercase();
            if !matches!(p, b'q' | b'r' | b'b' | b'n') {
                return Err("bad promotion piece".into());
            }
            out.promo = p;
        }
        Ok(out)
    }
}
