pub fn find_best_match(target: &str, candidates: impl Iterator<Item = String>) -> Option<String> {
    let mut best_candidate = None;
    let mut min_distance = usize::MAX;

    for candidate in candidates {
        let distance = levenshtein_distance(target, &candidate);
        // Only consider it a match if distance is small enough relative to the word length
        // e.g. distance <= 3 and at least some similarity
        let threshold = if target.len() < 3 { 1 } else { 3 };

        if distance <= threshold && distance < min_distance {
            min_distance = distance;
            best_candidate = Some(candidate);
        }
    }

    best_candidate
}

fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let s1_len = s1.chars().count();
    let s2_len = s2.chars().count();
    let mut matrix = vec![vec![0; s2_len + 1]; s1_len + 1];

    for i in 0..=s1_len {
        matrix[i][0] = i;
    }
    for j in 0..=s2_len {
        matrix[0][j] = j;
    }

    for (i, char1) in s1.chars().enumerate() {
        for (j, char2) in s2.chars().enumerate() {
            let cost = if char1 == char2 { 0 } else { 1 };
            matrix[i + 1][j + 1] = std::cmp::min(
                std::cmp::min(matrix[i][j + 1] + 1, matrix[i + 1][j] + 1),
                matrix[i][j] + cost,
            );
        }
    }

    matrix[s1_len][s2_len]
}
