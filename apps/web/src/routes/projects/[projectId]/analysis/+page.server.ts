import { fail, type Actions } from '@sveltejs/kit';
import { ApiClientError, cancelAnalysisJob } from '$lib/server/api';

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
