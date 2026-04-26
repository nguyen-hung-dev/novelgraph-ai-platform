export type Project = {
	id: string;
	workspace_id: string;
	name: string;
	visibility: string;
	created_at: string;
	updated_at: string;
};

export type Novel = {
	id: string;
	project_id: string;
	title: string;
	author: string | null;
	source_language: string | null;
	created_at: string;
	updated_at: string;
};

export type Chapter = {
	id: string;
	novel_id: string;
	chapter_num: number;
	title: string;
	content: string;
	created_at: string;
	updated_at: string;
};

export type AnalysisJob = {
	id: string;
	project_id: string;
	novel_id: string | null;
	job_type: string;
	status: string;
	payload_json: string;
	started_at: string | null;
	finished_at: string | null;
	error_code: string | null;
	error_message: string | null;
	created_at: string;
	updated_at: string;
};

export type JobEvent = {
	id: string;
	project_id: string;
	job_id: string;
	job_kind: string;
	sequence: number;
	event_type: string;
	payload_json: string;
	created_at: string;
};

export type ChapterPreview = {
	chapter_num: number;
	title: string;
	start_char: number;
	end_char: number;
	char_count: number;
	preview: string;
};

export type ImportPreview = {
	title: string;
	total_chars: number;
	chapter_count: number;
	chapters: ChapterPreview[];
};

export type NovelImportInput = {
	title: string;
	author?: string | null;
	source_language?: string | null;
	text: string;
};

export type NovelImportResult = {
	novel: Novel;
	chapters: Chapter[];
	source_segment_count: number;
	analysis_job: AnalysisJob;
};

export type ProjectWorkspaceSnapshot = {
	project: Project;
	novels: Novel[];
	active_novel: Novel | null;
	chapters: Chapter[];
	latest_analysis_job: AnalysisJob | null;
	latest_job_events: JobEvent[];
};

export type DeleteProjectResult = {
	project_id: string;
	action: string;
	data_retained: boolean;
};

export type LocalLlmModelSelection = {
	source_kind: string;
	display_name: string;
	path: string;
	preset_id: string | null;
	size_bytes: number | null;
	exists: boolean;
};

export type LocalLlmPreset = {
	id: string;
	name: string;
	description: string;
	filename: string;
	size_label: string;
	source_url: string;
	installed: boolean;
	active: boolean;
};

export type LocalLlmDownloadState = {
	preset_id: string;
	preset_name: string;
	filename: string;
	target_path: string;
	status: string;
	bytes_downloaded: number;
	total_bytes: number | null;
	auto_activate: boolean;
	error_message: string | null;
};

export type LocalLlmRuntimeSnapshot = {
	base_url: string;
	default_model_alias: string;
	server_binary: string;
	models_dir: string;
	server_running: boolean;
	server_pid: number | null;
	selected_model: LocalLlmModelSelection | null;
	downloaded_models: LocalLlmModelSelection[];
	presets: LocalLlmPreset[];
	active_download: LocalLlmDownloadState | null;
	last_error: string | null;
};

export type LocalLlmHealth = {
	provider: string;
	base_url: string;
	reachable: boolean;
	status_code: number | null;
	status_text: string | null;
};

export type ApiErrorEnvelope = {
	error: {
		code: string;
		message: string;
	};
};
