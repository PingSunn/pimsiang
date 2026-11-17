pub fn character_error_rate(reference: &str, hypothesis: &str) -> f64 {
    let reference: Vec<char> = reference.chars().filter(|c| !c.is_whitespace()).collect();
    let hypothesis: Vec<char> = hypothesis.chars().filter(|c| !c.is_whitespace()).collect();
    if reference.is_empty() {
        return if hypothesis.is_empty() { 0.0 } else { 1.0 };
    }

    let mut previous: Vec<usize> = (0..=hypothesis.len()).collect();
    let mut current = vec![0; hypothesis.len() + 1];
    for (i, r) in reference.iter().enumerate() {
        current[0] = i + 1;
        for (j, h) in hypothesis.iter().enumerate() {
            let substitution = previous[j] + usize::from(r != h);
            current[j + 1] = substitution.min(previous[j + 1] + 1).min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[hypothesis.len()] as f64 / reference.len() as f64
}
