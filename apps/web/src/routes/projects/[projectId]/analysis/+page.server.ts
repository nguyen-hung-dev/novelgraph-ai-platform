import { fail, type Actions } from '@sveltejs/kit';
import { ApiClientError, cancelAnalysisJob, getAnalysisRun } from '$lib/server/api';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ fetch, params, parent }) => {
	const projectId = params.projectId;
	const { workspace } = await parent();
	const latestJob = workspace.latest_analysis_job;

	if (!projectId || !latestJob) {
		return {
			analysisRun: null,
			analysisRunError: null
		};
	}

	try {
		return {
			analysisRun: await getAnalysisRun(fetch, projectId, latestJob.id),
			analysisRunError: null
		};
	} catch (error) {
		return {
			analysisRun: null,
			analysisRunError:
				error instanceof ApiClientError ? error.message : 'Không thể nạp tiến độ phân tích.'
		};
	}
};

export const actions: Actions = {
	cancelJob: async ({ fetch, params, request }) => {
		const projectId = params.projectId;
		if (!projectId) {
			return fail(404, {
				cancelJob: {
					error: 'Không tìm thấy project.'
				}
			});
		}

		const formData = await request.formData();
		const jobId = String(formData.get('job_id') ?? '').trim();

		if (!jobId) {
			return fail(400, {
				cancelJob: {
					error: 'Thiếu analysis job id.'
				}
			});
		}

		try {
			await cancelAnalysisJob(fetch, projectId, jobId);
			return {
				cancelJob: {
					ok: true
				}
			};
		} catch (error) {
			const status = error instanceof ApiClientError ? error.status : 400;
			const message =
				error instanceof ApiClientError ? error.message : 'Không thể hủy analysis job.';

			return fail(status, {
				cancelJob: {
					error: message
				}
			});
		}
	}
};
