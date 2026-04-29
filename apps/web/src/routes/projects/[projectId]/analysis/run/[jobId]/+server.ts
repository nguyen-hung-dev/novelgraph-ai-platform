import { error, json, type RequestHandler } from '@sveltejs/kit';
import {
	ApiClientError,
	getAnalysisRun,
	pauseAnalysisRun,
	resetAnalysisRun,
	stepAnalysisRun
} from '$lib/server/api';

function toHttpError(err: unknown): never {
	if (err instanceof ApiClientError) {
		error(err.status, err.message);
	}

	error(500, err instanceof Error ? err.message : 'Analysis run request failed.');
}

function requireParams(projectId: string | undefined, jobId: string | undefined) {
	if (!projectId || !jobId) {
		error(404, 'Analysis run not found.');
	}

	return { jobId, projectId };
}

export const GET: RequestHandler = async ({ fetch, params }) => {
	const { projectId, jobId } = requireParams(params.projectId, params.jobId);

	try {
		return json(await getAnalysisRun(fetch, projectId, jobId));
	} catch (err) {
		toHttpError(err);
	}
};

export const POST: RequestHandler = async ({ fetch, params, request }) => {
	const { projectId, jobId } = requireParams(params.projectId, params.jobId);
	const body = (await request.json().catch(() => ({}))) as {
		action?: string;
		force?: boolean;
		from_chapter_num?: number;
		to_chapter_num?: number;
		execution_profile?: 'local_small_staged' | 'cloud_gemini_one_shot';
	};

	try {
		if (body.action === 'reset') {
			return json(await resetAnalysisRun(fetch, projectId, jobId));
		}

		if (body.action === 'pause') {
			return json(await pauseAnalysisRun(fetch, projectId, jobId));
		}

		return json(
			await stepAnalysisRun(fetch, projectId, jobId, {
				force: Boolean(body.force),
				from_chapter_num: body.from_chapter_num,
				to_chapter_num: body.to_chapter_num,
				execution_profile: body.execution_profile
			})
		);
	} catch (err) {
		toHttpError(err);
	}
};
