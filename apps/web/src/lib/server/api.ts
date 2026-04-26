import { env } from '$env/dynamic/private';
import type {
	AnalysisJob,
	AnalysisRunSnapshot,
	ApiErrorEnvelope,
	DeleteProjectResult,
	ImportPreview,
	LocalLlmHealth,
	LocalLlmRuntimeSnapshot,
	NovelImportInput,
	NovelImportResult,
	Project,
	ProjectWorkspaceSnapshot
} from '$lib/api/types';

type FetchLike = typeof globalThis.fetch;

export class ApiClientError extends Error {
	status: number;
	code: string;

	constructor(status: number, code: string, message: string) {
		super(message);
		this.name = 'ApiClientError';
		this.status = status;
		this.code = code;
	}
}

function apiBaseUrl() {
	const raw = env.API_BASE_URL ?? env.VITE_API_BASE_URL ?? 'http://127.0.0.1:3000';

	return raw.endsWith('/') ? raw.slice(0, -1) : raw;
}

async function requestJson<T>(fetchFn: FetchLike, path: string, init?: RequestInit): Promise<T> {
	const response = await fetchFn(`${apiBaseUrl()}${path}`, {
		...init,
		headers: {
			accept: 'application/json',
			...(init?.body ? { 'content-type': 'application/json' } : {}),
			...(init?.headers ?? {})
		}
	});

	if (!response.ok) {
		const body = (await response.json().catch(() => null)) as ApiErrorEnvelope | null;

		throw new ApiClientError(
			response.status,
			body?.error.code ?? 'api_error',
			body?.error.message ?? `API request failed with HTTP ${response.status}`
		);
	}

	return (await response.json()) as T;
}

export async function listProjects(fetchFn: FetchLike) {
	return requestJson<Project[]>(fetchFn, '/api/projects');
}

export async function listArchivedProjects(fetchFn: FetchLike) {
	return requestJson<Project[]>(fetchFn, '/api/projects/archived');
}

export async function createProject(fetchFn: FetchLike, input: { name: string }) {
	return requestJson<Project>(fetchFn, '/api/projects', {
		method: 'POST',
		body: JSON.stringify(input)
	});
}

export async function deleteProject(
	fetchFn: FetchLike,
	projectId: string,
	input: { purge_data: boolean }
) {
	return requestJson<DeleteProjectResult>(fetchFn, `/api/projects/${projectId}`, {
		method: 'POST',
		body: JSON.stringify(input)
	});
}

export async function restoreProject(fetchFn: FetchLike, projectId: string) {
	return requestJson<Project>(fetchFn, `/api/projects/${projectId}/restore`, {
		method: 'POST'
	});
}

export async function getProjectWorkspace(fetchFn: FetchLike, projectId: string) {
	return requestJson<ProjectWorkspaceSnapshot>(fetchFn, `/api/projects/${projectId}/workspace`);
}

export async function previewNovelImport(
	fetchFn: FetchLike,
	projectId: string,
	input: NovelImportInput
) {
	return requestJson<ImportPreview>(fetchFn, `/api/projects/${projectId}/novels/import/preview`, {
		method: 'POST',
		body: JSON.stringify(input)
	});
}

export async function confirmNovelImport(
	fetchFn: FetchLike,
	projectId: string,
	input: NovelImportInput
) {
	return requestJson<NovelImportResult>(
		fetchFn,
		`/api/projects/${projectId}/novels/import/confirm`,
		{
			method: 'POST',
			body: JSON.stringify(input)
		}
	);
}

export async function cancelAnalysisJob(fetchFn: FetchLike, projectId: string, jobId: string) {
	return requestJson<AnalysisJob>(
		fetchFn,
		`/api/projects/${projectId}/analysis/jobs/${jobId}/cancel`,
		{
			method: 'POST'
		}
	);
}

export async function getAnalysisRun(fetchFn: FetchLike, projectId: string, jobId: string) {
	return requestJson<AnalysisRunSnapshot>(
		fetchFn,
		`/api/projects/${projectId}/analysis/jobs/${jobId}/run`
	);
}

export async function stepAnalysisRun(
	fetchFn: FetchLike,
	projectId: string,
	jobId: string,
	input: { force?: boolean; from_chapter_num?: number; to_chapter_num?: number } = {}
) {
	return requestJson<AnalysisRunSnapshot>(
		fetchFn,
		`/api/projects/${projectId}/analysis/jobs/${jobId}/run/step`,
		{
			method: 'POST',
			body: JSON.stringify({
				force: Boolean(input.force),
				from_chapter_num: input.from_chapter_num,
				to_chapter_num: input.to_chapter_num
			})
		}
	);
}

export async function resetAnalysisRun(fetchFn: FetchLike, projectId: string, jobId: string) {
	return requestJson<AnalysisRunSnapshot>(
		fetchFn,
		`/api/projects/${projectId}/analysis/jobs/${jobId}/run/reset`,
		{
			method: 'POST'
		}
	);
}

export async function pauseAnalysisRun(fetchFn: FetchLike, projectId: string, jobId: string) {
	return requestJson<AnalysisRunSnapshot>(
		fetchFn,
		`/api/projects/${projectId}/analysis/jobs/${jobId}/pause`,
		{
			method: 'POST'
		}
	);
}

export async function getLocalLlmRuntime(fetchFn: FetchLike) {
	return requestJson<LocalLlmRuntimeSnapshot>(fetchFn, '/api/local-llm/runtime');
}

export async function getLocalLlmHealth(fetchFn: FetchLike) {
	return requestJson<LocalLlmHealth>(fetchFn, '/api/local-llm/health');
}

export async function pickExistingLocalModel(fetchFn: FetchLike) {
	return requestJson<LocalLlmRuntimeSnapshot>(fetchFn, '/api/local-llm/runtime/select-existing', {
		method: 'POST'
	});
}

export async function startSelectedLocalModel(fetchFn: FetchLike) {
	return requestJson<LocalLlmRuntimeSnapshot>(fetchFn, '/api/local-llm/runtime/start-selected', {
		method: 'POST'
	});
}

export async function stopLocalLlmServer(fetchFn: FetchLike) {
	return requestJson<LocalLlmRuntimeSnapshot>(fetchFn, '/api/local-llm/runtime/stop', {
		method: 'POST'
	});
}

export async function activateManagedLocalModel(fetchFn: FetchLike, input: { path: string }) {
	return requestJson<LocalLlmRuntimeSnapshot>(fetchFn, '/api/local-llm/runtime/models/activate', {
		method: 'POST',
		body: JSON.stringify(input)
	});
}

export async function downloadPresetLocalModel(fetchFn: FetchLike, presetId: string) {
	return requestJson<LocalLlmRuntimeSnapshot>(
		fetchFn,
		`/api/local-llm/runtime/presets/${presetId}/download`,
		{
			method: 'POST'
		}
	);
}
