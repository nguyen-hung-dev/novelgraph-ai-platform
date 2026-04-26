import type { Chapter, JobEvent, ProjectWorkspaceSnapshot } from '$lib/api/types';
import type { Tone } from '$lib/workspace/demo';

export type ProjectNavItem = {
	id: string;
	name: string;
	stage: string;
	chapterCount: number;
};

export type ProjectCardView = {
	id: string;
	name: string;
	summary: string;
	sourceLanguage: string;
	chapterCount: number;
	wordCount: number;
	stage: string;
	updatedAt: string;
	tags: string[];
};

export function countWords(text: string) {
	const trimmed = text.trim();
	if (!trimmed) {
		return 0;
	}

	return trimmed.split(/\s+/).length;
}

export function totalWordCount(chapters: Chapter[]) {
	return chapters.reduce((total, chapter) => total + countWords(chapter.content), 0);
}

export function formatTimestamp(value: string) {
	const date = new Date(value);
	if (Number.isNaN(date.getTime())) {
		return value;
	}

	return date.toLocaleString('en-US', {
		month: 'short',
		day: 'numeric',
		hour: '2-digit',
		minute: '2-digit'
	});
}

export function deriveProjectStage(snapshot: ProjectWorkspaceSnapshot) {
	if (!snapshot.active_novel) {
		return 'No novel imported';
	}

	if (!snapshot.latest_analysis_job) {
		return 'Novel ready';
	}

	return snapshot.latest_analysis_job.status.replace(/_/g, ' ');
}

export function buildProjectNavItem(snapshot: ProjectWorkspaceSnapshot): ProjectNavItem {
	return {
		id: snapshot.project.id,
		name: snapshot.project.name,
		stage: deriveProjectStage(snapshot),
		chapterCount: snapshot.chapters.length
	};
}

export function buildProjectCard(snapshot: ProjectWorkspaceSnapshot): ProjectCardView {
	const chapterCount = snapshot.chapters.length;
	const wordCount = totalWordCount(snapshot.chapters);
	const sourceLanguage = snapshot.active_novel?.source_language ?? 'Not set';
	const stage = deriveProjectStage(snapshot);
	const summary = snapshot.active_novel
		? `${snapshot.active_novel.title} is loaded with ${chapterCount} chapters ready for reading and job inspection.`
		: 'Project created. Import a TXT or Markdown novel to start splitting chapters and creating analysis jobs.';
	const tags = [
		snapshot.active_novel ? 'Imported' : 'Empty',
		snapshot.latest_analysis_job
			? `Analysis ${snapshot.latest_analysis_job.status}`
			: 'No analysis job',
		sourceLanguage
	];

	return {
		id: snapshot.project.id,
		name: snapshot.project.name,
		summary,
		sourceLanguage,
		chapterCount,
		wordCount,
		stage,
		updatedAt: formatTimestamp(snapshot.project.updated_at),
		tags
	};
}

export function chapterParagraphs(chapter: Chapter) {
	return chapter.content
		.split(/\r?\n\r?\n/)
		.map((paragraph) => paragraph.trim())
		.filter(Boolean);
}

export function prettyEventLabel(eventType: string) {
	return eventType.replace(/_/g, ' ');
}

export function summarizeEventPayload(event: JobEvent) {
	try {
		const payload = JSON.parse(event.payload_json) as Record<string, unknown>;
		const pairs = Object.entries(payload)
			.map(([key, value]) => `${key}: ${String(value)}`)
			.slice(0, 3);

		return pairs.join(' · ');
	} catch {
		return event.payload_json;
	}
}

export function jobStatusTone(status: string): Tone {
	switch (status) {
		case 'completed':
			return 'good';
		case 'running':
			return 'teal';
		case 'paused':
			return 'warning';
		case 'failed':
			return 'danger';
		case 'cancelled':
			return 'warning';
		case 'pending':
		default:
			return 'neutral';
	}
}
