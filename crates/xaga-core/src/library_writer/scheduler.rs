use super::*;

/// Cap on concurrently executing lane writes. Writes are ms-scale file RMWs; the cap
/// bounds spawn pressure during bulk intake (feed refresh) without approaching the
/// design-scale corpus limits — lanes above the cap simply wait their turn.
const MAX_IN_FLIGHT: usize = 8;

/// Per-file lane: FIFO queue of pending ops (same-file order is submission order,
/// regardless of tier — an Interactive op never jumps a Regular op targeting the SAME
/// file, because the earlier op may create the file the later one needs).
struct Lane {
    queue: VecDeque<(Tier, Op)>,
    busy: bool,
    /// Whether this lane's key currently sits in one of the ready lists.
    listed: bool,
}

pub(super) async fn dispatch_loop(
    library_dir: PathBuf,
    rx: flume::Receiver<Msg>,
    token: WriteToken,
) {
    let mut lanes: HashMap<String, Lane> = HashMap::new();
    // Cross-lane priority: Interactive-headed lanes dispatch before Regular-headed ones.
    let mut ready_interactive: VecDeque<String> = VecDeque::new();
    let mut ready_regular: VecDeque<String> = VecDeque::new();
    let mut in_flight: usize = 0;
    let mut shutdown_ack: Option<oneshot::Sender<()>> = None;

    let (completion_tx, completion_rx) = flume::unbounded::<String>();

    loop {
        // While shutting down, drain any buffered messages BEFORE dispatch so the
        // interactive backlog reaches the lanes and gets dispatched below. Draining
        // first is essential: a message that was sent (ok) but not yet received still
        // holds its responder, so acking with messages left in the channel would strand
        // those callers forever. `submit` fast-fails once shutdown begins, so the inflow
        // is bounded and this converges. Regular is re-derivable (tier contract) — refused.
        if shutdown_ack.is_some() {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    Msg::Submit(Tier::Interactive, op) => intake(
                        &mut lanes,
                        &mut ready_interactive,
                        &mut ready_regular,
                        Tier::Interactive,
                        *op,
                    ),
                    Msg::Submit(Tier::Regular, op) => {
                        log::info!("library writer: shutdown — refusing {}", op.label());
                        op.refuse();
                    }
                    Msg::Shutdown(_) => {}
                }
            }
        }

        // Dispatch every ready lane up to the concurrency cap, Interactive first.
        while in_flight < MAX_IN_FLIGHT {
            let Some(key) = ready_interactive
                .pop_front()
                .or_else(|| ready_regular.pop_front())
            else {
                break;
            };
            let lane = lanes.get_mut(&key).expect("listed lane exists");
            lane.listed = false;
            let (_, op) = lane.queue.pop_front().expect("listed lane has work");
            lane.busy = true;
            in_flight += 1;

            let dir = library_dir.clone();
            let done = completion_tx.clone();
            let lane_key = key.clone();
            tokio::spawn(async move {
                op.execute(&dir, token).await;
                let _ = done.send(lane_key);
            });
        }

        // Shutdown completes only once the drained interactive backlog has run out and
        // nothing is queued or in flight.
        if let Some(ack) = shutdown_ack.take() {
            if in_flight == 0 && lanes.values().all(|l| l.queue.is_empty() && !l.busy) {
                let _ = ack.send(());
                return;
            }
            shutdown_ack = Some(ack);
        }

        tokio::select! {
            msg = rx.recv_async() => match msg {
                Ok(Msg::Submit(tier, op)) => {
                    if shutdown_ack.is_some() && tier == Tier::Regular {
                        // Drop policy: Regular is re-derivable by tier contract.
                        log::info!("library writer: shutdown — refusing {}", op.label());
                        op.refuse();
                        continue;
                    }
                    intake(&mut lanes, &mut ready_interactive, &mut ready_regular, tier, *op);
                }
                Ok(Msg::Shutdown(ack)) => {
                    shutdown_ack = Some(ack);
                    purge_regular(&mut lanes, &mut ready_interactive, &mut ready_regular);
                }
                // Every handle dropped: exit once in-flight work lands. Pending queued
                // ops can have no listeners, but complete them anyway — they were
                // accepted, and durability beats speed on the way out.
                Err(_) => {
                    if in_flight == 0 && lanes.values().all(|l| l.queue.is_empty() && !l.busy) {
                        return;
                    }
                }
            },
            key = completion_rx.recv_async() => {
                let key = key.expect("completion channel outlives loop");
                in_flight -= 1;
                let lane = lanes.get_mut(&key).expect("completing lane exists");
                lane.busy = false;
                if lane.queue.is_empty() {
                    lanes.remove(&key);
                } else {
                    relist(&mut lanes, &mut ready_interactive, &mut ready_regular, key);
                }
            }
        }
    }
}

fn intake(
    lanes: &mut HashMap<String, Lane>,
    ready_interactive: &mut VecDeque<String>,
    ready_regular: &mut VecDeque<String>,
    tier: Tier,
    op: Op,
) {
    let key = op.lane_key();
    let lane = lanes.entry(key.clone()).or_insert_with(|| Lane {
        queue: VecDeque::new(),
        busy: false,
        listed: false,
    });
    lane.queue.push_back((tier, op));
    if !lane.busy && !lane.listed {
        lane.listed = true;
        match lane.queue.front().expect("just pushed").0 {
            Tier::Interactive => ready_interactive.push_back(key),
            Tier::Regular => ready_regular.push_back(key),
        }
    }
}

/// Re-list a lane whose head changed (previous op completed), by its new head's tier.
fn relist(
    lanes: &mut HashMap<String, Lane>,
    ready_interactive: &mut VecDeque<String>,
    ready_regular: &mut VecDeque<String>,
    key: String,
) {
    let lane = lanes.get_mut(&key).expect("relisting lane exists");
    lane.listed = true;
    match lane.queue.front().expect("relist requires work").0 {
        Tier::Interactive => ready_interactive.push_back(key),
        Tier::Regular => ready_regular.push_back(key),
    }
}

/// Shutdown drop: refuse every queued Regular op, then rebuild the ready lists around
/// what remains (all Interactive). In-flight ops are untouched — they complete.
fn purge_regular(
    lanes: &mut HashMap<String, Lane>,
    ready_interactive: &mut VecDeque<String>,
    ready_regular: &mut VecDeque<String>,
) {
    let mut dropped = 0usize;
    for lane in lanes.values_mut() {
        let mut kept = VecDeque::with_capacity(lane.queue.len());
        for (tier, op) in lane.queue.drain(..) {
            match tier {
                Tier::Interactive => kept.push_back((tier, op)),
                Tier::Regular => {
                    dropped += 1;
                    log::info!("library writer: shutdown — dropping queued {}", op.label());
                    op.refuse();
                }
            }
        }
        lane.queue = kept;
        lane.listed = false;
    }
    ready_interactive.clear();
    ready_regular.clear();
    lanes.retain(|_, lane| lane.busy || !lane.queue.is_empty());
    let keys: Vec<String> = lanes
        .iter()
        .filter(|(_, l)| !l.busy && !l.queue.is_empty())
        .map(|(k, _)| k.clone())
        .collect();
    for key in keys {
        relist(lanes, ready_interactive, ready_regular, key);
    }
    if dropped > 0 {
        log::info!("library writer: shutdown — dropped {dropped} Regular write(s) (re-derivable)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn shutdown_refuses_regular_backlog_and_retains_interactive_lane_order() {
        let mut lanes = HashMap::new();
        let mut interactive = VecDeque::new();
        let mut regular = VecDeque::new();
        let mut replies = Vec::new();
        for (tier, title) in [
            (Tier::Regular, "rebuildable"),
            (Tier::Interactive, "first"),
            (Tier::Interactive, "second"),
        ] {
            let (resp, reply) = oneshot::channel();
            intake(
                &mut lanes,
                &mut interactive,
                &mut regular,
                tier,
                Op::SaveArticle {
                    id: "same-entry".into(),
                    url: "https://example.org/entry".into(),
                    title: title.into(),
                    published_at: None,
                    updated_at: None,
                    summary: None,
                    favicon: None,
                    markdown: String::new(),
                    url_data: HashMap::new(),
                    resp,
                },
            );
            replies.push(reply);
        }
        purge_regular(&mut lanes, &mut interactive, &mut regular);
        assert!(matches!(
            replies.remove(0).await.unwrap(),
            Err(WriterError::ShuttingDown)
        ));
        assert!(regular.is_empty());
        assert_eq!(interactive.len(), 1);
        let lane = lanes.values().next().unwrap();
        let retained: Vec<_> = lane
            .queue
            .iter()
            .map(|(tier, op)| {
                assert_eq!(*tier, Tier::Interactive);
                let Op::SaveArticle { title, .. } = op else {
                    panic!("unexpected operation")
                };
                title.as_str()
            })
            .collect();
        assert_eq!(retained, ["first", "second"]);
        for mut reply in replies {
            assert!(matches!(
                reply.try_recv(),
                Err(tokio::sync::oneshot::error::TryRecvError::Empty)
            ));
        }
    }
}
