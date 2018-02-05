use aes::*;
use codec::*;
use oracle::*;
use pkcs::*;
use rand;

use nom::*;
use nom::IResult::*;
use nom::Needed::Size;

use std::collections::HashMap;
use std::str::from_utf8;
use std::io::Write;

pub fn set_2() {
    _9();
    _10();
    _11();
    _12();
    _13();
    _14();
    _15();
}

fn _9() {
    let x = b"YELLOW SUBMARINE";
    assert_eq!(b"YELLOW SUBMARINE\x04\x04\x04\x04", &pkcs7(x, 20).unwrap()[..]);
}

fn _10() {
    let file = include_bytes!("../data/10.txt");
    let dec = base64_decode_filter(file);
    let res = aes128_cbc_decode_pad(&dec, *b"YELLOW SUBMARINE", [0; 16]).unwrap();
    assert_eq!(from_utf8(&res).unwrap(), include_str!("../data/7_result.txt"));
}

fn _11() {
    // There is exactly one pair of identical 16-byte chunks in this file,
    // so we can detect whether it's ECB-encrypted.
    let plaintext = include_bytes!("../data/7_result.txt");
    for _ in 0..10 {
        let (mystery, mode) = random_encrypt(plaintext);
        assert_eq!(ecb_cbc_oracle(&mystery[..]), mode);
    }
}

type Oracle = Fn(&[u8]) -> Vec<u8>;

fn ecb_block_size(ora: &Oracle) -> usize {
    let bytes = &[b'A'; 255];
    let mut out_length = None;
    let mut block_size = None;
    for i in 1..=255 {
        let enc = ora(&bytes[..i]);
        match out_length {
            Some(l) if enc.len() > l => {
                block_size = Some(enc.len() - l);
                break;
            }
            Some(_) => { },
            None => { out_length = Some(enc.len()); }
        }
    }
    block_size.unwrap()
}

fn is_ecb(ora: &Oracle, block_size: usize) -> bool {
    let ecb_test = vec![b'A'; block_size * 2];
    let ecb_test_out = ora(&ecb_test[..]);
    ecb_test_out[..block_size] == ecb_test_out[block_size..block_size * 2]
}

fn bytes_by_english_freq() -> Vec<u8> {
    let mut freq = Vec::new();
    freq.extend_from_slice(b" ");
    freq.extend_from_slice(b"etaoinshrdlcumwfgypbvkjxqz");
    freq.extend_from_slice(b"0123456789");
    freq.extend_from_slice(b"\n.,!?-'\"/");
    freq.extend_from_slice(b"ETAOINSHRDLCUMWFGYPBVKJXQZ");
    freq.extend_from_slice(&(0u8..=255).collect::<Vec<u8>>()[..]);
    freq
}

// This needs some work.
// prefix_length is the string ora prepends to its input (possibly 0).
// If it's 0, this works out to always pad with 16 bytes, though that's
// unnecessary.
// Otherwise, start_block gets the index of the first block of text we
// have full control over.
fn break_ecb_with_oracle(ora: &Oracle, block_size: usize, prefix_length: usize) -> Vec<u8> {
    // We want to always pad with this many bytes no matter what
    // so that the rest is block-aligned.
    let const_pad = block_size - (prefix_length % block_size);
    
    let start_block = prefix_length / block_size + 1;

    // Iterating last_byte over freq is a little faster than iterating over
    // 0..=255, but it's not necessary.
    let mut freq: Vec<u8> = bytes_by_english_freq();
    
    let mut known_bytes = Vec::new();

    // We want to look at each block of the unknown plaintext in turn!
    // If there's an unknown prefix, we want to start at the first block that
    // we have full control over.
    'outer: for block_ix in start_block.. {
        for offs in 1..=block_size {
            let pad_width = block_size - offs;
            let mut padding = vec![b'A'; pad_width + const_pad];

            // Get the actual value of the block.
            let actual = &ora(&padding[..])[
                block_size * block_ix..
                block_size * (block_ix + 1)
            ];

            // Make a dictionary of possible values for this block.
            //
            // here, we only need to pass ONE BLOCK into the oracle.
            // We just need to make sure the first block_size - 1 bytes of
            // that block are equal to the first block_size - 1 bytes of the
            // plaintext of the `actual` block.
            let mut d = vec![b'A'; block_size];
            d.extend_from_slice(&known_bytes[..]);
            let mut dict_padding = vec![b'A'; const_pad];
            let slice_start = known_bytes.len() + 1;
            dict_padding.extend_from_slice(&d[slice_start..slice_start + block_size - 1]);
            dict_padding.push(0);

            assert_eq!(dict_padding[..const_pad], b"AAAAAAAAAAAAAAAA"[..const_pad]);
            assert_eq!(dict_padding.len() - const_pad, block_size);

            let mut matched = false;
            for last_byte in freq.iter() {
                dict_padding[const_pad + block_size - 1] = *last_byte;
                let this_option = &ora(&dict_padding[..])[
                    block_size * start_block..
                    block_size * (start_block + 1)
                ];
                assert_eq!(this_option.len(), actual.len());
                if this_option == actual {
                    matched = true;
                    known_bytes.push(*last_byte);
                    break;
                }
            }
            if !matched {
                break 'outer;
            }
        }
    }
    known_bytes
}

fn _12() {
    // Get block size
    let key: [u8; 16] = rand::random();

    let unknown = base64_decode_filter(
        include_bytes!("../data/12.txt"));

    let ora: &Oracle = &(move |buffer| {
        let mut v = Vec::new();
        v.extend_from_slice(buffer);
        v.extend_from_slice(&unknown[..]);
        aes128_ecb_encode_pad(&v[..], key)
    });
    let block_size = ecb_block_size(ora);

    // Detect that oracle uses ecb
    assert!(is_ecb(ora, block_size));

    // Break it
    let answer = break_ecb_with_oracle(ora, block_size, 0);
    println!("{}", from_utf8(&answer[..]).unwrap());
}

type KVVec<'a, 'b> = Vec<(&'a [u8], &'b [u8])>;

named!(key,
    take_until_and_consume!(&b"="[..]));
named!(value,
    take_until_and_consume!(&b"&"[..]));
named!(kvpair< (&[u8], &[u8]) >,
    tuple!(key, value));
named!(kvs<KVVec>, many0!(kvpair));

fn sanitize(bytes: &[u8]) -> Vec<u8> {
    bytes.iter()
        .cloned()
        .filter(|&ch| ch != b'=' && ch != b'&')
        .collect()
}

fn mk_profile(email: &[u8]) -> KVVec {
    let mut h = Vec::new();
    h.push((&b"email"[..], &email[..]));
    h.push((&b"uid"[..],   &b"10"[..]));
    h.push((&b"role"[..],  &b"user"[..]));
    h
}

fn url_encode(obj: KVVec) -> Vec<u8> {
    let mut out = Vec::new();
    for (k, v) in obj {
        out.extend_from_slice(k);
        out.push(b'=');
        out.extend_from_slice(v);
        out.push(b'&');
    }
    out.pop();
    out
}

fn mk_encrypted_url_profile(email: &[u8], key: [u8; 16]) -> Vec<u8> {
    let obj = mk_profile(email);
    let url = url_encode(obj);
    aes128_ecb_encode_pad(&url[..], key)
}

fn pretty_ct(ciphertext: &[u8]) {
    for chunk in ciphertext.chunks(16) {
        print!("{} ", from_utf8(&base16_encode(chunk)[..]).unwrap());
    }
    println!("");
}

// Everything here is hardcoded because I'm lazy but you can do this even if
// you don't know the prefix (which in this case is `email=`) and you could
// decode the entire postfix to determine that the padding on `to` needs to
// be 13
fn _13() {
    let key = rand::random();
    let oracle: &Oracle = &(move |bytes| {
        let mut v = Vec::new();
        v.extend_from_slice(bytes);
        mk_encrypted_url_profile(&v[..], key)
    });

    let block_size = ecb_block_size(oracle);
    let prefix_length = guess_prefix_length(oracle, block_size);

    let pad_length = block_size - (prefix_length % block_size);
    let pad_blocks = 1 + (prefix_length / block_size);

    // We're setting our email address such that one of the blocks of the
    // ciphertext will be "admin", padded using EKCS7.
    // Since we computed the prefix length, we know exactly which
    // ciphertext block that is.
    let mut evil_email = vec![b'A'; pad_length];
    evil_email.extend_from_slice(
        b"admin\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b"
    );
    let evil = oracle(&evil_email[..]);

    pretty_ct(&evil[..]);

    // We're setting our email address such that the last block of the
    // ciphertext will be "user____________" (EKCS7 padded) as in "role=user"
    // Since we just figured out the ciphertext of "admin___________" (EKCS7
    // padded), we can just paste it over that last block and the result will
    // decrypt successfully.
    let mut to = oracle(&[b'A'; 13][..]);
    let last = to.len() - 16;
    to[last..].copy_from_slice(&evil[pad_blocks*block_size..(pad_blocks+1)*block_size]);

    pretty_ct(&to[..]);

    let dec = aes128_ecb_decode_pad(&to[..], key).unwrap();
    println!("{}", from_utf8(&dec[..]).unwrap());
    match kvs(&dec) {
        Done(_, o) => println!("{:?}", o),
        _ => println!("failed")
    }
}

// Return the first block that matches the next one, and its position in units
// of block_size.
fn adj_blocks_match(known_pt: &[u8], block_size: usize, oracle: &Oracle) -> Option<(usize, Vec<u8>)> {
    let mut match_exists = false;
    let mut bytes = oracle(known_pt);
    let block_count = bytes.len() / block_size;
    for i in 0..block_count-1 {
        let first = &bytes[block_size*i..block_size*(i+1)];
        let second = &bytes[block_size*(i+1)..block_size*(i+2)];
        if  first == second {
            return Some((i, first.to_vec()));
        }
    }
    None
}

fn guess_prefix_length(oracle: &Oracle, block_size: usize) -> usize {
    // Passing this into the oracle is guaranteed to result in two adjacent
    // matching blocks, so we can safely unwrap.
    let mut prefix_test = vec![b'A'; block_size*3];
    let res = adj_blocks_match(&prefix_test[..], block_size, oracle).unwrap();
    let ix = res.0;
    let to_match = res.1;

    // (To be safe, we should repeat prefix_test with vec![b'B'; block_size*3]
    // which will let us confirm where our injected text is going in case
    // the prefix or the plaintext results in false positives.
    // For instance, there's a chance that the prefix contains matching
    // adjacent blocks, or that the last fractional block of prefix text is
    // all b'A'.)

    // Let's remove elements until we stop having two adjacent matching blocks
    // We gotta be careful here because adj_blocks_match might find a match
    // due to the actual unknown plaintext, but we only want to find matches
    // due to the attacker-supplied plaintext.
    while Some(&to_match) == adj_blocks_match(&prefix_test[..], block_size, oracle).map(|x| x.1).as_ref() {
        prefix_test.pop();
    }
    let n = prefix_test.len() + 1;
    let prefix_pad = n % block_size;
    let length = ix * block_size - prefix_pad;

    length
}

fn _14() {
    // This whole chunk just constructs the oracle from the challenge
    // description: take some bytes, prepend with an unknown random prefix
    // of some length between 0 and 255, append an unknown message,
    // encrypt the whole thing under ECB under a random key.
    // Given that the same key, prefix, and message are used each time,
    // we can decrypt the message.
    let key = rand::random();
    let prefix_length: u8 = rand::random();
    let mut prefix = Vec::new();
    for i in 0..prefix_length {
        prefix.push(rand::random());
    }
    let unknown = base64_decode_filter(
        include_bytes!("../data/12.txt"));
    let oracle: &Oracle = &(move |bytes| {
        let mut v = Vec::new();
        v.extend_from_slice(&prefix[..]);
        v.extend_from_slice(bytes);
        v.extend_from_slice(&unknown[..]);
        aes128_ecb_encode_pad(&v[..], key)
    });

    // Let's figure out how long the prefix is.
    // To do that, we first need to determine the block size
    let block_size = ecb_block_size(oracle);
    let length = guess_prefix_length(oracle, block_size);

    // Knowing the prefix length, we can break it much like we would break
    // a similar oracle with no prefix.
    let res = break_ecb_with_oracle(oracle, block_size, length);
    let unpad_res = undo_pkcs7(&res[..]);

    assert_eq!(&include_bytes!("../data/rollin.txt")[..], &unpad_res[..]);
}

fn _15() {
    assert!(undo_pkcs7_checked(b"ICE ICE BABY\x04\x04\x04\x04").is_some());

    assert!(undo_pkcs7_checked(b"ICE ICE BABY\x05\x05\x05\x05").is_none());
    assert!(undo_pkcs7_checked(b"ICE ICE BABY\x01\x02\x03\x04").is_none());
}
