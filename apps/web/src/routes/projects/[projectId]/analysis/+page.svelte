<script lang="ts">
	import { invalidateAll } from '$app/navigation';
	import { resolve } from '$app/paths';
	import MetricCard from '$lib/components/MetricCard.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import type { AnalysisRunSnapshot } from '$lib/api/types';
	import {
		formatTimestamp,
		jobStatusTone,
		prettyEventLabel,
		summarizeEventPayload
	} from '$lib/workspace/presenters';
	import type { ActionData, PageData } from './$types';

	let { data, form }: { data: PageData; form?: ActionData } = $props();

	let runSnapshot = $state<AnalysisRunSnapshot | null>(null);
	let runnerState = $state<'idle' | 'running' | 'pause_requested' | 'paused'>('idle');
	let runnerNote = $state<string | null>(null);
	let pauseRequested = false;
	let runFromChapter = $state('');
	let runToChapter = $state('');
	let rangeDefaultsKey = $state('');

	type RunRequestContext = {
		endpoint: string;
		jobId: string;
	};

	type ChapterRunRange = {
		from_chapter_num: number;
		to_chapter_num: number;
	};

	$effect(() => {
		if (runnerState !== 'running') {
			runSnapshot = data.analysisRun;
			runnerNote = data.analysisRunError;
		}
	});

	$effect(() => {
		const nextChapter = data.analysisRun?.next_chapter_num ?? 1;
		const lastChapter = data.workspace.chapters.at(-1)?.chapter_num ?? nextChapter;
		const nextDefaultsKey = `${data.workspace.project.id}:${nextChapter}:${lastChapter}`;

		if (
			rangeDefaultsKey !== nextDefaultsKey &&
			runnerState !== 'running' &&
			runnerState !== 'pause_requested'
		) {
			runFromChapter = String(nextChapter);
			runToChapter = String(lastChapter);
			rangeDefaultsKey = nextDefaultsKey;
		}
	});

	const currentRun = $derived(runSnapshot ?? data.analysisRun);
	const latestJob = $derived(currentRun?.job ?? data.workspace.latest_analysis_job);
	const latestEvents = $derived(data.workspace.latest_job_events);
	const completedCount = $derived(currentRun?.completed_chapters ?? 0);
	const totalCount = $derived(currentRun?.total_chapters ?? data.workspace.chapters.length);
	const failedCount = $derived(currentRun?.failed_chapters ?? 0);
	const pendingCount = $derived(currentRun?.pending_chapters ?? totalCount);
	const characterRecords = $derived(currentRun?.character_records ?? []);
	const progressPercent = $derived(
		totalCount > 0 ? Math.round((completedCount / totalCount) * 100) : 0
	);
	const canCancel = $derived(
		latestJob
			? latestJob.status === 'pending' ||
					latestJob.status === 'running' ||
					latestJob.status === 'paused'
			: false
	);
	const canRun = $derived(
		Boolean(
			latestJob &&
			runnerState !== 'running' &&
			runnerState !== 'pause_requested' &&
			latestJob.status !== 'cancelled'
		)
	);
	const canForce = $derived(
		Boolean(latestJob && runnerState !== 'running' && runnerState !== 'pause_requested')
	);
	const canPause = $derived(Boolean(latestJob && runnerState === 'running'));
	const cancelJobError = $derived(
		form?.cancelJob && 'error' in form.cancelJob ? form.cancelJob.error : null
	);
	const cancelJobOk = $derived(
		Boolean(form?.cancelJob && 'ok' in form.cancelJob && form.cancelJob.ok)
	);

	function chapterStatusTone(status: string) {
		if (status === 'completed') return 'good';
		if (status === 'running') return 'teal';
		if (status === 'failed') return 'danger';
		if (status === 'paused') return 'warning';
		return 'neutral';
	}

	function metricToneForJobStatus(status: string) {
		if (status === 'completed') return 'accent';
		if (status === 'running') return 'teal';
		if (status === 'failed' || status === 'cancelled') return 'rose';
		if (status === 'paused') return 'amber';
		return 'neutral';
	}

	function endpointFor(jobId: string) {
		return resolve(`/projects/${data.workspace.project.id}/analysis/run/${jobId}`);
	}

	function buildRunContext(): RunRequestContext {
		if (!latestJob) {
			throw new Error('Chưa có analysis job để chạy.');
		}

		return {
			endpoint: endpointFor(latestJob.id),
			jobId: latestJob.id
		};
	}

	function buildRunRange(): ChapterRunRange {
		const from = Number.parseInt(runFromChapter, 10);
		const to = Number.parseInt(runToChapter, 10);

		if (!Number.isInteger(from) || !Number.isInteger(to)) {
			throw new Error('Khoảng chương phải là số nguyên.');
		}

		if (from < 1 || to < 1) {
			throw new Error('Khoảng chương phải bắt đầu từ 1 trở lên.');
		}

		if (to < from) {
			throw new Error('Chương kết thúc phải lớn hơn hoặc bằng chương bắt đầu.');
		}

		return {
			from_chapter_num: from,
			to_chapter_num: to
		};
	}

	function rangeLabel(range: ChapterRunRange) {
		return range.from_chapter_num === range.to_chapter_num
			? `chương ${range.from_chapter_num}`
			: `chương ${range.from_chapter_num} -> ${range.to_chapter_num}`;
	}

	function confidenceLabel(confidence: number | null) {
		return confidence === null ? 'n/a' : `${Math.round(confidence * 100)}%`;
	}

	async function requestRun(
		context: RunRequestContext,
		action: 'pause' | 'reset' | 'step',
		force = false,
		range?: ChapterRunRange
	) {
		const response = await fetch(context.endpoint, {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ action, force, ...range })
		});

		if (!response.ok) {
			const text = await response.text();
			throw new Error(text || `Analysis request failed with HTTP ${response.status}`);
		}

		return (await response.json()) as AnalysisRunSnapshot;
	}

	async function runLoop(force = false) {
		if (!latestJob || runnerState === 'running' || runnerState === 'pause_requested') {
			return;
		}

		const context = buildRunContext();
		let range: ChapterRunRange;
		try {
			range = buildRunRange();
		} catch (error) {
			runnerNote = error instanceof Error ? error.message : 'Khoảng chương không hợp lệ.';
			return;
		}

		pauseRequested = false;
		runnerState = 'running';
		runnerNote = force
			? `Đang chạy lại ${rangeLabel(range)} và ghi đè trạng thái chạy cũ trong phạm vi này.`
			: `Đang chạy ${rangeLabel(range)}, các chương đã completed sẽ được bỏ qua.`;

		let forceNextStep = force;

		while (!pauseRequested) {
			try {
				const snapshot = await requestRun(context, 'step', forceNextStep, range);
				forceNextStep = false;
				runSnapshot = snapshot;

				if (pauseRequested) {
					break;
				}

				if (snapshot.job.status === 'paused') {
					runnerState = 'paused';
					runnerNote =
						snapshot.paused_reason ?? snapshot.job.error_message ?? 'Analysis đã tự tạm dừng.';
					await invalidateAll();
					return;
				}

				if (snapshot.job.status === 'completed' || snapshot.next_chapter_num === null) {
					runnerState = 'idle';
					runnerNote = 'Đã chạy xong toàn bộ chương trong job hiện tại.';
					await invalidateAll();
					return;
				}

				await new Promise((resolveDelay) => setTimeout(resolveDelay, 250));
			} catch (error) {
				runnerState = 'paused';
				runnerNote =
					error instanceof Error
						? `Tự tạm dừng vì mất kết nối hoặc request lỗi: ${error.message}`
						: 'Tự tạm dừng vì mất kết nối backend.';
				return;
			}
		}

		if (
			runSnapshot?.job.status === 'completed' ||
			(runSnapshot && runSnapshot.next_chapter_num === null)
		) {
			runnerState = 'idle';
			runnerNote = 'Đã chạy xong toàn bộ chương trong job hiện tại.';
			await invalidateAll();
			return;
		}

		try {
			runSnapshot = await requestRun(context, 'pause');
			runnerNote = 'Đã tạm dừng sau chương hiện tại.';
			await invalidateAll();
		} catch (error) {
			runnerNote =
				error instanceof Error
					? `Đã dừng vòng chạy UI, nhưng chưa ghi được trạng thái pause lên backend: ${error.message}`
					: 'Đã dừng vòng chạy UI, nhưng chưa ghi được trạng thái pause lên backend.';
		} finally {
			runnerState = 'paused';
		}
	}

	async function pauseRun() {
		if (!latestJob) {
			return;
		}

		pauseRequested = true;
		runnerState = 'pause_requested';
		runnerNote =
			'Đã nhận lệnh Pause. Request AI của chương hiện tại chưa bị cắt ngang; runner sẽ dừng sau khi request này trả về.';
	}

	function forceRun() {
		let range: ChapterRunRange;
		try {
			range = buildRunRange();
		} catch (error) {
			runnerNote = error instanceof Error ? error.message : 'Khoảng chương không hợp lệ.';
			return;
		}

		if (
			!window.confirm(
				`Chạy lại ${rangeLabel(range)} sẽ xóa trạng thái chapter run cũ trong phạm vi này.`
			)
		) {
			return;
		}

		void runLoop(true);
	}
</script>

<div class="page-stack">
	<section class="metrics-grid">
		<MetricCard
			detail="Chương completed trong analysis job hiện tại"
			label="Progress"
			tone="accent"
			value={`${completedCount}/${totalCount}`}
		/>
		<MetricCard
			detail="Resume sẽ chạy từ chương này và bỏ qua các chương completed"
			label="Next chapter"
			tone="teal"
			value={currentRun?.next_chapter_num ? `#${currentRun.next_chapter_num}` : 'None'}
		/>
		<MetricCard
			detail={`${pendingCount} pending · ${failedCount} failed`}
			label="Chapter states"
			tone={failedCount > 0 ? 'rose' : 'amber'}
			value={`${progressPercent}%`}
		/>
		<MetricCard
			detail={data.workspace.active_novel?.title ?? 'Import truyện trước'}
			label="Status"
			tone={latestJob ? metricToneForJobStatus(latestJob.status) : 'rose'}
			value={latestJob?.status ?? 'Idle'}
		/>
	</section>

	<section class="page-grid">
		<Panel
			subtitle="Điều khiển analysis theo từng chương bằng local llama.cpp"
			title="Analysis runner"
		>
			{#if latestJob}
				<div class="detail-list">
					<div class="status-row">
						<StatusPill label={latestJob.status} tone={jobStatusTone(latestJob.status)} />
						<StatusPill label={latestJob.job_type} />
						<StatusPill
							label={`UI ${runnerState}`}
							tone={runnerState === 'running' ? 'teal' : 'warning'}
						/>
					</div>

					<div class="progress-rail" aria-label="Analysis progress">
						<span style={`width: ${progressPercent}%`}></span>
					</div>

					<div class="callout-box">
						<div class="nav-link__title">{latestJob.id}</div>
						<div class="nav-link__meta">
							Created {formatTimestamp(latestJob.created_at)} · Updated
							{formatTimestamp(latestJob.updated_at)}
						</div>
					</div>

					{#if runnerNote || currentRun?.paused_reason || latestJob.error_message}
						<div class="warning-box">
							<div class="nav-link__title">Runtime note</div>
							<div class="nav-link__meta">
								{runnerNote ?? currentRun?.paused_reason ?? latestJob.error_message}
							</div>
						</div>
					{/if}

					{#if cancelJobError}
						<div class="warning-box">
							<div class="nav-link__title">Cancel job failed</div>
							<div class="nav-link__meta">{cancelJobError}</div>
						</div>
					{/if}
					{#if cancelJobOk}
						<div class="callout-box">
							<div class="nav-link__title">Cancel request accepted</div>
							<div class="nav-link__meta">Trang sẽ nạp lại trạng thái mới sau request.</div>
						</div>
					{/if}

					<div class="form-grid">
						<label class="form-field">
							<span class="field-label">Từ chương</span>
							<input
								bind:value={runFromChapter}
								disabled={runnerState === 'running' || runnerState === 'pause_requested'}
								inputmode="numeric"
								min="1"
								type="text"
							/>
						</label>
						<label class="form-field">
							<span class="field-label">Đến chương</span>
							<input
								bind:value={runToChapter}
								disabled={runnerState === 'running' || runnerState === 'pause_requested'}
								inputmode="numeric"
								min="1"
								type="text"
							/>
						</label>
					</div>

					<div class="table-actions">
						<button class="action-button" disabled={!canRun} onclick={() => void runLoop(false)}>
							{latestJob.status === 'paused' || runnerState === 'paused'
								? 'Resume'
								: 'Start / chạy tiếp'}
						</button>
						<button class="secondary-button" disabled={!canPause} onclick={() => void pauseRun()}>
							Pause
						</button>
						<button class="secondary-button" disabled={!canForce} onclick={forceRun}>
							Force rerun
						</button>
						{#if canCancel}
							<form action="?/cancelJob" method="POST">
								<input name="job_id" type="hidden" value={latestJob.id} />
								<button class="secondary-button" disabled={runnerState === 'running'} type="submit">
									Cancel job
								</button>
							</form>
						{/if}
						<a class="toolbar-link" href={resolve('/settings')}>Local model settings</a>
					</div>

					<div class="callout-box">
						<div class="nav-link__title">Run policy</div>
						<div class="nav-link__meta">
							Nếu Từ chương và Đến chương giống nhau, runner chỉ chạy chương đó. Nếu Đến chương
							lớn hơn Từ chương, runner chạy lần lượt trong phạm vi đã chọn và bỏ qua chương đã
							completed. Force rerun chỉ xóa trạng thái chapter run cũ trong phạm vi này. Nếu
							backend hoặc llama.cpp mất kết nối, UI sẽ tự tạm dừng vòng chạy; nếu lỗi đến từ
							llama.cpp, backend cũng ghi job về paused.
						</div>
					</div>
				</div>
			{:else}
				<div class="empty-note">
					Chưa có analysis job nào cho project này. Xác nhận import truyện sẽ tạo pending job đầu
					tiên.
				</div>
			{/if}
		</Panel>

		<div class="list-stack">
			<Panel subtitle="Operator-readable event trail" title="Progress events">
				{#if latestEvents.length > 0}
					<div class="event-stack">
						{#each latestEvents as event (event.id)}
							<div class="event-row">
								<div>
									<div class="nav-link__title">{prettyEventLabel(event.event_type)}</div>
									<div class="nav-link__meta">{summarizeEventPayload(event)}</div>
								</div>
								<StatusPill label={`#${event.sequence}`} />
							</div>
						{/each}
					</div>
				{:else}
					<div class="empty-note">Chưa có event nào cho job này.</div>
				{/if}
			</Panel>
		</div>
	</section>

	<Panel subtitle="Chapter-level run state" title="Chapter progress">
		{#if currentRun && currentRun.chapters.length > 0}
			<table class="table">
				<thead>
					<tr>
						<th>Chapter</th>
						<th>Status</th>
						<th>Attempt</th>
						<th>Updated</th>
						<th>Note</th>
					</tr>
				</thead>
				<tbody>
					{#each currentRun.chapters as chapter (chapter.chapter_id)}
						<tr>
							<td>
								<div class="nav-link__title">#{chapter.chapter_num} · {chapter.title}</div>
								<div class="nav-link__meta">{chapter.chapter_id}</div>
							</td>
							<td>
								<StatusPill label={chapter.status} tone={chapterStatusTone(chapter.status)} />
							</td>
							<td>{chapter.attempt ?? '-'}</td>
							<td>{chapter.updated_at ? formatTimestamp(chapter.updated_at) : '-'}</td>
							<td>
								{#if chapter.error_message}
									<span class="nav-link__meta">{chapter.error_message}</span>
								{:else if chapter.prompt_schema_version}
									<span class="nav-link__meta">{chapter.prompt_schema_version}</span>
								{:else}
									<span class="nav-link__meta">Chưa chạy.</span>
								{/if}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		{:else if data.analysisRunError}
			<div class="warning-box">
				<div class="nav-link__title">Không thể nạp chapter progress</div>
				<div class="nav-link__meta">{data.analysisRunError}</div>
			</div>
		{:else}
			<div class="empty-note">Chưa có chapter nào để phân tích.</div>
		{/if}
	</Panel>

	<Panel subtitle="Parsed records stored from the character extraction schema" title="Nhân vật">
		{#if characterRecords.length > 0}
			<div class="character-grid">
				{#each characterRecords as record (record.id)}
					<article class="info-card">
						<div class="status-row">
							<div>
								<div class="nav-link__title">
									#{record.chapter_num} · {record.display_name}
								</div>
								<div class="nav-link__meta">
									{record.group_label} · {record.entity_key ?? 'Chưa có entity key'} ·
									{record.prompt_schema_version}
								</div>
							</div>
							<StatusPill label={record.group_key} tone="teal" />
						</div>

						{#if record.fields.length > 0}
							<div class="field-stack">
								{#each record.fields as field (field.id)}
									<div class="field-row">
										<div class="field-row__label">
											<span class="field-label">{field.field_label}</span>
											<span class="nav-link__meta">{field.field_key}</span>
										</div>
										<div class="field-row__values">
											{#each field.values as value (value.id)}
												<div class="callout-box">
													<div class="nav-link__title">{value.value}</div>
													<div class="nav-link__meta">
														Confidence {confidenceLabel(value.confidence)}
													</div>
													{#if value.evidence.length > 0}
														<div class="evidence-stack">
															{#each value.evidence as evidence}
																<div class="nav-link__meta">
																	{#if evidence.quote}
																		"{evidence.quote}"
																	{/if}
																	{#if evidence.reason}
																		· {evidence.reason}
																	{/if}
																</div>
															{/each}
														</div>
													{/if}
												</div>
											{/each}
										</div>
									</div>
								{/each}
							</div>
						{:else}
							<div class="empty-note">Record này chưa có field nhỏ.</div>
						{/if}
					</article>
				{/each}
			</div>
		{:else}
			<div class="empty-note">
				Chưa có dữ liệu nhân vật đã parse trong DB. Chạy analysis cho một chương để tạo record
				`character`.
			</div>
		{/if}
	</Panel>
</div>
