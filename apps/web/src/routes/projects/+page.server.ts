import { fail, redirect, type Actions } from '@sveltejs/kit';
import {
	ApiClientError,
	createProject,
	deleteProject,
	listArchivedProjects,
	restoreProject
} from '$lib/server/api';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ fetch }) => {
	try {
		return {
			archivedProjects: await listArchivedProjects(fetch)
		};
	} catch (error) {
		const message =
			error instanceof ApiClientError ? error.message : 'Không thể nạp archived projects.';

		return {
			archivedProjects: [],
			archivedProjectsError: message
		};
	}
};

export const actions: Actions = {
	createProject: async ({ fetch, request }) => {
		const formData = await request.formData();
		const name = String(formData.get('name') ?? '').trim();

		if (!name) {
			return fail(400, {
				createProject: {
					error: 'Tên project là bắt buộc.',
					name
				}
			});
		}

		let projectPath: string;
		try {
			const project = await createProject(fetch, { name });
			projectPath = `/projects/${project.id}/import`;
		} catch (error) {
			const message =
				error instanceof ApiClientError ? error.message : 'Không thể tạo project lúc này.';

			return fail(400, {
				createProject: {
					error: message,
					name
				}
			});
		}

		redirect(303, projectPath);
	},
	deleteProject: async ({ fetch, request }) => {
		const formData = await request.formData();
		const projectId = String(formData.get('project_id') ?? '').trim();
		const purgeData = formData.get('purge_data') !== null;

		if (!projectId) {
			return fail(400, {
				deleteProject: {
					error: 'Thiếu project id.',
					projectId,
					purgeData
				}
			});
		}

		try {
			await deleteProject(fetch, projectId, { purge_data: purgeData });
		} catch (error) {
			const message =
				error instanceof ApiClientError ? error.message : 'Không thể xóa project lúc này.';

			return fail(400, {
				deleteProject: {
					error: message,
					projectId,
					purgeData
				}
			});
		}

		redirect(303, '/projects');
	},
	restoreProject: async ({ fetch, request }) => {
		const formData = await request.formData();
		const projectId = String(formData.get('project_id') ?? '').trim();

		if (!projectId) {
			return fail(400, {
				restoreProject: {
					error: 'Thiếu project id.',
					projectId
				}
			});
		}

		try {
			await restoreProject(fetch, projectId);
		} catch (error) {
			const message =
				error instanceof ApiClientError ? error.message : 'Không thể khôi phục project lúc này.';

			return fail(400, {
				restoreProject: {
					error: message,
					projectId
				}
			});
		}

		redirect(303, '/projects');
	}
};
