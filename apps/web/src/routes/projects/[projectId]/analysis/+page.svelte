<script lang="ts">
	import { browser } from '$app/environment';
	import { invalidateAll } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';
	import MetricCard from '$lib/components/MetricCard.svelte';
	import Panel from '$lib/components/Panel.svelte';
	import StatusPill from '$lib/components/StatusPill.svelte';
	import { connectProjectRealtime } from '$lib/api/realtime';
	import type { AnalysisRunSnapshot } from '$lib/api/types';
	import {
		ANALYSIS_COPY,
		ANALYSIS_PROFILE_OPTIONS,
		analysisChapterStatesDetail,
		analysisForceRerunConfirm,
		analysisLostConnectionNote,
		analysisRequestFailedMessage,
		analysisRunStartedNote,
		type AnalysisExecutionProfile,
		type ChapterRunRange
	} from '$lib/workspace/analysisCopy';
	import { formatTimestamp, jobStatusTone } from '$lib/workspace/presenters';
	import type { ActionData, PageData } from './$types';

	let { data, form }: { data: PageData; form?: ActionData } = $props();

	let runSnapshot = $state<AnalysisRunSnapshot | null>(null);
	let runnerState = $state<'idle' | 'running' | 'pause_requested' | 'paused'>('idle');
	let runnerNote = $state<string | null>(null);
	let pauseRequested = false;
	let runFromChapter = $state('');
	let runToChapter = $state('');
	let rangeDefaultsKey = $state('');

	onMount(() => {
		if (!browser) {
			return;
		}

		return connectProjectRealtime(data.workspace.project.id, (event) => {
			if (event.event_type === 'connected') {
				return;
			}
			if (document.visibilityState === 'visible') {
				void invalidateAll();
			}
		});
	});

	type RunRequestContext = {
		endpoint: string;
		jobId: string;
	};

	let executionProfile = $state<AnalysisExecutionProfile>('local_small_staged');

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
	const completedCount = $derived(currentRun?.completed_chapters ?? 0);
	const totalCount = $derived(currentRun?.total_chapters ?? data.workspace.chapters.length);
	const failedCount = $derived(currentRun?.failed_chapters ?? 0);
	const pendingCount = $derived(currentRun?.pending_chapters ?? totalCount);
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
	const latestTelemetryChapter = $derived(
		[...(currentRun?.chapters ?? [])]
			.reverse()
			.find((chapter) => chapter.api_call_count != null || chapter.provider != null)
	);

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
			throw new Error(ANALYSIS_COPY.errors.missingJob);
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
			throw new Error(ANALYSIS_COPY.errors.invalidRangeInteger);
		}

		if (from < 1 || to < 1) {
			throw new Error(ANALYSIS_COPY.errors.invalidRangeStart);
		}

		if (to < from) {
			throw new Error(ANALYSIS_COPY.errors.invalidRangeOrder);
		}

		return {
			from_chapter_num: from,
			to_chapter_num: to
		};
	}

	async function requestRun(
		context: RunRequestContext,
		action: 'pause' | 'reset' | 'step',
		force = false,
		range?: ChapterRunRange,
		profile?: AnalysisExecutionProfile
	) {
		const response = await fetch(context.endpoint, {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({
				action,
				force,
				...range,
				execution_profile: profile
			})
		});

		if (!response.ok) {
			const text = await response.text();
			throw new Error(text || analysisRequestFailedMessage(response.status));
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
			runnerNote =
				error instanceof Error ? error.message : ANALYSIS_COPY.errors.invalidRangeFallback;
			return;
		}

		pauseRequested = false;
		runnerState = 'running';
		runnerNote = analysisRunStartedNote(force, range, executionProfile);

		let forceNextStep = force;

		while (!pauseRequested) {
			try {
				const snapshot = await requestRun(
					context,
					'step',
					forceNextStep,
					range,
					executionProfile
				);
				forceNextStep = false;
				runSnapshot = snapshot;

				if (pauseRequested) {
					break;
				}

				if (snapshot.job.status === 'paused') {
					runnerState = 'paused';
					runnerNote =
						snapshot.paused_reason ??
						snapshot.job.error_message ??
						ANALYSIS_COPY.errors.autoPaused;
					await invalidateAll();
					return;
				}

				if (snapshot.job.status === 'completed' || snapshot.next_chapter_num === null) {
					runnerState = 'idle';
					runnerNote = ANALYSIS_COPY.notes.jobCompleted;
					await invalidateAll();
					return;
				}

				await new Promise((resolveDelay) => setTimeout(resolveDelay, 250));
			} catch (error) {
				runnerState = 'paused';
				runnerNote =
					error instanceof Error
						? analysisLostConnectionNote(error.message)
						: analysisLostConnectionNote();
				return;
			}
		}

		if (
			runSnapshot?.job.status === 'completed' ||
			(runSnapshot && runSnapshot.next_chapter_num === null)
		) {
			runnerState = 'idle';
			runnerNote = ANALYSIS_COPY.notes.jobCompleted;
			await invalidateAll();
			return;
		}

		try {
			runSnapshot = await requestRun(context, 'pause');
			runnerNote = ANALYSIS_COPY.notes.pausedAfterChapter;
			await invalidateAll();
		} catch (error) {
			runnerNote =
				error instanceof Error
					? `${ANALYSIS_COPY.notes.pauseWriteFailed}: ${error.message}`
					: ANALYSIS_COPY.notes.pauseWriteFailed;
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
		runnerNote = ANALYSIS_COPY.notes.pauseQueued;
	}

	function forceRun() {
		let range: ChapterRunRange;
		try {
			range = buildRunRange();
		} catch (error) {
			runnerNote =
				error instanceof Error ? error.message : ANALYSIS_COPY.errors.invalidRangeFallback;
			return;
		}

		if (!window.confirm(analysisForceRerunConfirm(range))) {
			return;
		}

		void runLoop(true);
	}
</script>

<div class="page-stack">
	<section class="metrics-grid">
		<MetricCard
			detail={ANALYSIS_COPY.metrics.progress.detail}
			label={ANALYSIS_COPY.metrics.progress.label}
			tone="accent"
			value={`${completedCount}/${totalCount}`}
		/>
		<MetricCard
			detail={ANALYSIS_COPY.metrics.nextChapter.detail}
			label={ANALYSIS_COPY.metrics.nextChapter.label}
			tone="teal"
			value={currentRun?.next_chapter_num
				? `#${currentRun.next_chapter_num}`
				: ANALYSIS_COPY.metrics.nextChapter.none}
		/>
		<MetricCard
			detail={analysisChapterStatesDetail(pendingCount, failedCount)}
			label={ANALYSIS_COPY.metrics.chapterStates.label}
			tone={failedCount > 0 ? 'rose' : 'amber'}
			value={`${progressPercent}%`}
		/>
		<MetricCard
			detail={data.workspace.active_novel?.title ?? ANALYSIS_COPY.metrics.status.fallbackDetail}
			label={ANALYSIS_COPY.metrics.status.label}
			tone={latestJob ? metricToneForJobStatus(latestJob.status) : 'rose'}
			value={latestJob?.status ?? ANALYSIS_COPY.metrics.status.idle}
		/>
	</section>

	<section class="page-grid">
		<Panel subtitle={ANALYSIS_COPY.runner.subtitle} title={ANALYSIS_COPY.runner.title}>
			{#if latestJob}
				<div class="detail-list">
					<div class="status-row">
						<StatusPill label={latestJob.status} tone={jobStatusTone(latestJob.status)} />
						<StatusPill label={latestJob.job_type} />
						<StatusPill
							label={`${ANALYSIS_COPY.runner.uiStatusPrefix} ${runnerState}`}
							tone={runnerState === 'running' ? 'teal' : 'warning'}
						/>
					</div>

					<div class="progress-rail" aria-label={ANALYSIS_COPY.runner.progressAria}>
						<span style={`width: ${progressPercent}%`}></span>
					</div>

					<div class="callout-box">
						<div class="nav-link__title">{latestJob.id}</div>
						<div class="nav-link__meta">
							{ANALYSIS_COPY.runner.createdLabel}
							{formatTimestamp(latestJob.created_at)} · {ANALYSIS_COPY.runner.updatedLabel}
							{formatTimestamp(latestJob.updated_at)}
						</div>
					</div>

					{#if runnerNote || currentRun?.paused_reason || latestJob.error_message}
						<div class="warning-box">
							<div class="nav-link__title">{ANALYSIS_COPY.runner.runtimeNoteTitle}</div>
							<div class="nav-link__meta">
								{runnerNote ?? currentRun?.paused_reason ?? latestJob.error_message}
							</div>
						</div>
					{/if}

					{#if cancelJobError}
						<div class="warning-box">
							<div class="nav-link__title">{ANALYSIS_COPY.runner.cancelFailedTitle}</div>
							<div class="nav-link__meta">{cancelJobError}</div>
						</div>
					{/if}
					{#if cancelJobOk}
						<div class="callout-box">
							<div class="nav-link__title">{ANALYSIS_COPY.runner.cancelAcceptedTitle}</div>
							<div class="nav-link__meta">{ANALYSIS_COPY.runner.cancelAcceptedMeta}</div>
						</div>
					{/if}

					<div class="form-grid">
						<label class="form-field">
							<span class="field-label">{ANALYSIS_COPY.runner.fromChapterLabel}</span>
							<input
								bind:value={runFromChapter}
								disabled={runnerState === 'running' || runnerState === 'pause_requested'}
								inputmode="numeric"
								min="1"
								type="text"
							/>
						</label>
						<label class="form-field">
							<span class="field-label">{ANALYSIS_COPY.runner.toChapterLabel}</span>
							<input
								bind:value={runToChapter}
								disabled={runnerState === 'running' || runnerState === 'pause_requested'}
								inputmode="numeric"
								min="1"
								type="text"
							/>
						</label>
						<label class="form-field">
							<span class="field-label">{ANALYSIS_COPY.runner.profileLabel}</span>
							<select
								bind:value={executionProfile}
								disabled={runnerState === 'running' || runnerState === 'pause_requested'}
							>
								{#each ANALYSIS_PROFILE_OPTIONS as option}
									<option value={option.value}>{option.label}</option>
								{/each}
							</select>
						</label>
					</div>

					<div class="table-actions">
						<button class="action-button" disabled={!canRun} onclick={() => void runLoop(false)}>
							{latestJob.status === 'paused' || runnerState === 'paused'
								? ANALYSIS_COPY.runner.resumeButton
								: ANALYSIS_COPY.runner.startButton}
						</button>
						<button class="secondary-button" disabled={!canPause} onclick={() => void pauseRun()}>
							{ANALYSIS_COPY.runner.pauseButton}
						</button>
						<button class="secondary-button" disabled={!canForce} onclick={forceRun}>
							{ANALYSIS_COPY.runner.forceButton}
						</button>
						{#if canCancel}
							<form action="?/cancelJob" method="POST">
								<input name="job_id" type="hidden" value={latestJob.id} />
								<button class="secondary-button" disabled={runnerState === 'running'} type="submit">
									{ANALYSIS_COPY.runner.cancelButton}
								</button>
							</form>
						{/if}
						<a class="toolbar-link" href={resolve('/settings')}>{ANALYSIS_COPY.runner.settingsLink}</a>
					</div>

					{#if latestTelemetryChapter}
						<div class="callout-box">
							<div class="nav-link__title">{ANALYSIS_COPY.telemetry.title}</div>
							<div class="nav-link__meta">
								{ANALYSIS_COPY.telemetry.profile}
								<code>{latestTelemetryChapter.execution_profile ?? ANALYSIS_COPY.telemetry.empty}</code>
								· {ANALYSIS_COPY.telemetry.status}
								<code>{latestTelemetryChapter.call_status ?? ANALYSIS_COPY.telemetry.empty}</code>
								· {ANALYSIS_COPY.telemetry.calls}
								<code>{latestTelemetryChapter.api_call_count ?? ANALYSIS_COPY.telemetry.empty}</code>
							</div>
							<div class="nav-link__meta">
								{ANALYSIS_COPY.telemetry.provider}
								<code>{latestTelemetryChapter.provider ?? ANALYSIS_COPY.telemetry.empty}</code> ·
								{ANALYSIS_COPY.telemetry.model}
								<code>{latestTelemetryChapter.model ?? ANALYSIS_COPY.telemetry.empty}</code> ·
								{ANALYSIS_COPY.telemetry.tokens}
								<code
									>{latestTelemetryChapter.input_tokens ?? 0}/{latestTelemetryChapter.output_tokens ??
										0}</code
								>
							</div>
						</div>
					{/if}

					<div class="callout-box">
						<div class="nav-link__title">{ANALYSIS_COPY.runPolicy.title}</div>
						<div class="nav-link__meta">{ANALYSIS_COPY.runPolicy.body}</div>
					</div>
				</div>
			{:else}
				<div class="empty-note">{ANALYSIS_COPY.runner.emptyNote}</div>
			{/if}
		</Panel>
	</section>

</div>
