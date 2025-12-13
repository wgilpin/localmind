use localmind_rs::{db::Database, Result};
use rand::seq::SliceRandom;
use rand::thread_rng;

#[derive(Debug)]
struct BoundaryIssue {
    doc_id: i64,
    doc_title: String,
    chunk_id: i64,
    chunk_index: usize,
    issue_type: String,
    context: String,
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '\'' || c == '-'
}

fn check_start_boundary(text: &str, chunk_start: usize, doc_content: &str) -> Option<String> {
    // Skip if at document start
    if chunk_start == 0 {
        return None;
    }

    // Get character before chunk start
    let chars: Vec<char> = doc_content.chars().collect();
    if chunk_start >= chars.len() {
        return Some("Chunk start beyond document length".to_string());
    }

    let prev_char = if chunk_start > 0 {
        chars[chunk_start - 1]
    } else {
        return None;
    };

    let first_char = text.chars().next().unwrap_or(' ');

    // Check if we're starting mid-word
    if is_word_char(prev_char) && is_word_char(first_char) {
        // Get context: 10 chars before and after
        let context_start = chunk_start.saturating_sub(10);
        let context_end = (chunk_start + 10).min(chars.len());
        let context: String = chars[context_start..context_end].iter().collect();
        let marker_pos = (chunk_start - context_start).min(context.len());

        return Some(format!(
            "Starts mid-word: '{}' | '{}' (prev: '{}', curr: '{}')",
            &context[..marker_pos],
            &context[marker_pos..],
            prev_char,
            first_char
        ));
    }

    None
}

fn check_end_boundary(text: &str, chunk_end: usize, doc_content: &str) -> Option<String> {
    let chars: Vec<char> = doc_content.chars().collect();

    // Skip if at document end
    if chunk_end >= chars.len() {
        return None;
    }

    // Get last char of chunk and next char after chunk
    let last_char = text.chars().last().unwrap_or(' ');
    let next_char = chars[chunk_end];

    // Check if we're ending mid-word
    if is_word_char(last_char) && is_word_char(next_char) {
        // Get context: 10 chars before and after
        let context_start = chunk_end.saturating_sub(10);
        let context_end = (chunk_end + 10).min(chars.len());
        let context: String = chars[context_start..context_end].iter().collect();
        let marker_pos = (chunk_end - context_start).min(context.len());

        return Some(format!(
            "Ends mid-word: '{}' | '{}' (last: '{}', next: '{}')",
            &context[..marker_pos],
            &context[marker_pos..],
            last_char,
            next_char
        ));
    }

    None
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("LocalMind Chunk Boundary Assessment Tool");
    println!("============================================");
    println!();

    // Initialize database
    let db = Database::new().await?;

    // Get all documents
    println!("Loading documents from database...");
    let documents = db.get_all_documents().await?;

    if documents.is_empty() {
        println!("ℹ️ No documents found in database");
        return Ok(());
    }

    println!("Found {} documents", documents.len());

    // Collect all chunks from all documents
    println!("Loading all chunks...");
    let mut all_chunks = Vec::new();

    for doc in &documents {
        let chunks = db.get_chunk_embeddings_for_document(doc.id).await?;
        for chunk_data in chunks {
            let (chunk_id, chunk_index, chunk_start, chunk_end, _embedding) = chunk_data;
            all_chunks.push((
                doc.id,
                doc.title.clone(),
                doc.content.clone(),
                chunk_id,
                chunk_index,
                chunk_start,
                chunk_end,
            ));
        }
    }

    println!("Total chunks: {}", all_chunks.len());

    // Randomly sample 100 chunks
    let sample_size = 100.min(all_chunks.len());
    println!("Randomly sampling {} chunks...", sample_size);

    let mut rng = thread_rng();
    all_chunks.shuffle(&mut rng);
    let sample = &all_chunks[..sample_size];

    println!();
    println!("Analyzing chunk boundaries...");
    println!();

    let mut issues = Vec::new();
    let mut start_issues = 0;
    let mut end_issues = 0;
    let mut both_issues = 0;
    let mut no_issues = 0;

    // Allow up to 15 characters beyond the end for word boundary leeway
    const BOUNDARY_LEEWAY: usize = 15;

    for (i, (doc_id, doc_title, doc_content, chunk_id, chunk_index, chunk_start, chunk_end)) in
        sample.iter().enumerate()
    {
        // Extract chunk text
        let content_len = doc_content.len(); // byte length

        if *chunk_end > content_len + BOUNDARY_LEEWAY {
            println!(
                "⚠️  Chunk {}/{}: Invalid boundaries ({}..{} > {} + {})",
                i + 1,
                sample_size,
                chunk_start,
                chunk_end,
                content_len,
                BOUNDARY_LEEWAY
            );
            continue;
        }

        // Clamp chunk_end to actual content length for extraction
        let actual_chunk_end = (*chunk_end).min(content_len);
        let chunk_text =
            std::str::from_utf8(&doc_content.as_bytes()[*chunk_start..actual_chunk_end])
                .unwrap_or("")
                .to_string();

        // Check start boundary
        let start_issue = check_start_boundary(&chunk_text, *chunk_start, doc_content);

        // Check end boundary
        let end_issue = check_end_boundary(&chunk_text, *chunk_end, doc_content);

        // Categorize and record issues
        match (&start_issue, &end_issue) {
            (Some(start_msg), Some(end_msg)) => {
                both_issues += 1;
                issues.push(BoundaryIssue {
                    doc_id: *doc_id,
                    doc_title: doc_title.chars().take(40).collect(),
                    chunk_id: *chunk_id,
                    chunk_index: *chunk_index,
                    issue_type: "BOTH".to_string(),
                    context: format!("START: {} | END: {}", start_msg, end_msg),
                });
                println!(
                    "Chunk {}/{}: Doc '{}' Chunk #{} - BOTH BOUNDARIES BAD",
                    i + 1,
                    sample_size,
                    doc_title.chars().take(30).collect::<String>(),
                    chunk_index
                );
            }
            (Some(start_msg), None) => {
                start_issues += 1;
                issues.push(BoundaryIssue {
                    doc_id: *doc_id,
                    doc_title: doc_title.chars().take(40).collect(),
                    chunk_id: *chunk_id,
                    chunk_index: *chunk_index,
                    issue_type: "START".to_string(),
                    context: start_msg.clone(),
                });
                println!(
                    "⚠️  Chunk {}/{}: Doc '{}' Chunk #{} - START BOUNDARY BAD",
                    i + 1,
                    sample_size,
                    doc_title.chars().take(30).collect::<String>(),
                    chunk_index
                );
            }
            (None, Some(end_msg)) => {
                end_issues += 1;
                issues.push(BoundaryIssue {
                    doc_id: *doc_id,
                    doc_title: doc_title.chars().take(40).collect(),
                    chunk_id: *chunk_id,
                    chunk_index: *chunk_index,
                    issue_type: "END".to_string(),
                    context: end_msg.clone(),
                });
                println!(
                    "⚠️  Chunk {}/{}: Doc '{}' Chunk #{} - END BOUNDARY BAD",
                    i + 1,
                    sample_size,
                    doc_title.chars().take(30).collect::<String>(),
                    chunk_index
                );
            }
            (None, None) => {
                no_issues += 1;
                if (i + 1) % 10 == 0 {
                    println!("Chunk {}/{}: Good boundaries", i + 1, sample_size);
                }
            }
        }
    }

    println!();
    println!("============================================");
    println!("BOUNDARY ASSESSMENT REPORT");
    println!("============================================");
    println!();
    println!("Sample size: {}", sample_size);
    println!();
    println!(
        "Good boundaries:       {} ({:.1}%)",
        no_issues,
        (no_issues as f64 / sample_size as f64) * 100.0
    );
    println!(
        "⚠️  Start issues only:    {} ({:.1}%)",
        start_issues,
        (start_issues as f64 / sample_size as f64) * 100.0
    );
    println!(
        "⚠️  End issues only:      {} ({:.1}%)",
        end_issues,
        (end_issues as f64 / sample_size as f64) * 100.0
    );
    println!(
        "Both boundaries bad:   {} ({:.1}%)",
        both_issues,
        (both_issues as f64 / sample_size as f64) * 100.0
    );
    println!();
    println!(
        "Total issues:            {} ({:.1}%)",
        issues.len(),
        (issues.len() as f64 / sample_size as f64) * 100.0
    );
    println!();

    if !issues.is_empty() {
        println!("============================================");
        println!("DETAILED ISSUE REPORT");
        println!("============================================");
        println!();

        // Show first 10 issues
        let show_count = 10.min(issues.len());
        println!("Showing first {} of {} issues:", show_count, issues.len());
        println!();

        for (i, issue) in issues.iter().take(show_count).enumerate() {
            println!("Issue #{}", i + 1);
            println!("  Doc ID: {} - '{}'", issue.doc_id, issue.doc_title);
            println!("  Chunk: #{} (ID: {})", issue.chunk_index, issue.chunk_id);
            println!("  Type: {}", issue.issue_type);
            println!("  Context: {}", issue.context);
            println!();
        }

        if issues.len() > show_count {
            println!("... and {} more issues", issues.len() - show_count);
            println!();
        }

        println!("============================================");
        println!("RECOMMENDATION");
        println!("============================================");
        println!();

        let issue_rate = (issues.len() as f64 / sample_size as f64) * 100.0;

        if issue_rate > 50.0 {
            println!(
                "CRITICAL: {:.1}% of chunks have boundary issues!",
                issue_rate
            );
            println!("   Action: Run the rechunk utility to fix all chunks:");
            println!("   cargo run --bin rechunk");
        } else if issue_rate > 20.0 {
            println!(
                "⚠️  WARNING: {:.1}% of chunks have boundary issues",
                issue_rate
            );
            println!("   Consider running the rechunk utility:");
            println!("   cargo run --bin rechunk");
        } else if issue_rate > 5.0 {
            println!(
                "ℹ️  MINOR: {:.1}% of chunks have boundary issues",
                issue_rate
            );
            println!("   This is acceptable but could be improved.");
        } else {
            println!(
                "EXCELLENT: Only {:.1}% of chunks have boundary issues",
                issue_rate
            );
            println!("   Chunk boundaries are well-aligned!");
        }
    } else {
        println!("PERFECT: All sampled chunks have good word boundaries!");
    }

    println!();

    Ok(())
}
