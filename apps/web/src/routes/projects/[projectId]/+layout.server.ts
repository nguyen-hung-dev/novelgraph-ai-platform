import { error } from '@sveltejs/kit';
import { ApiClientError, getProjectWorkspace } from '$lib/server/api';
import { buildProjectCard } from '$lib/workspace/presenters';
import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = async ({ fetch, params }) => {
	if (!params.projectId) {
		error(404, 'Project not found');
	}

	try {
		const workspace = await getProjectWorkspace(fetch, params.projectId);

		return {
			projectView: buildProjectCard(workspace),
			workspace
		};
	} catch (err) {
		if (err instanceof ApiClientError && err.status === 404) {
			error(404, 'Project not found');
		}

		throw err;
	}
};
