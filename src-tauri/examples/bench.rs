use num_bigint::BigUint;
use num_traits::One;
use std::time::Instant;

const BITS: usize = 640 * 416 * 12;

fn main() {
    println!("бит: {}", BITS);
    println!(
        "десятичных цифр: ~{}",
        (BITS as f64 * std::f64::consts::LOG10_2) as usize
    );

    // Один случайный множитель нужной длины.
    let mut bytes = vec![0u8; BITS / 8];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = (i * 37 + 11) as u8;
    }
    let a = BigUint::from_bytes_be(&bytes) | BigUint::one();

    let t = Instant::now();
    let p = &a * &a;
    println!("умножение 3.2M бит: {:?} (бит: {})", t.elapsed(), p.bits());

    // Модульное возведение в степень (маскирование).
    let mask = (BigUint::one() << BITS) - BigUint::one();
    let t = Instant::now();
    let r = (&a * &a) & &mask;
    println!("умножение + маска: {:?} (бит: {})", t.elapsed(), r.bits());

    // Обратный элемент по Ньютону с растущей точностью.
    let t = Instant::now();
    let mut x = a.clone();
    let mut prec = 3usize;
    while prec < BITS {
        let new_prec = (prec * 2).min(BITS);
        let a_mask = &a & ((BigUint::one() << new_prec) - BigUint::one());
        let ax = (&a_mask * &x) & &((BigUint::one() << new_prec) - BigUint::one());
        let factor = (&((BigUint::one() << new_prec) - BigUint::one()) + BigUint::from(2u32)) - &ax;
        x = (&x * &factor) & &((BigUint::one() << new_prec) - BigUint::one());
        prec = new_prec;
    }
    println!("обратный по Ньютону: {:?}", t.elapsed());

    // Проверка обратимости.
    let check = (&a * &x) & &mask;
    println!("a * a^-1 == 1 (mod 2^k): {}", check == BigUint::one());

    // Десятичное представление — ключевая операция для UI (локация цифрами).
    let t = Instant::now();
    let dec = a.to_str_radix(10);
    println!("to_str_radix(10): {:?} len={}", t.elapsed(), dec.len());

    let t = Instant::now();
    let back = BigUint::parse_bytes(dec.as_bytes(), 10).unwrap();
    println!("parse_bytes(10): {:?} ok={}", t.elapsed(), back == a);

    let t = Instant::now();
    let hex = a.to_str_radix(16);
    println!("to_str_radix(16): {:?} len={}", t.elapsed(), hex.len());
}
