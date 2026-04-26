import type { LayoutServerLoad } from './$types';
import { getProjectWorkspace, listProjects } from '$lib/server/api';
import { buildProjectCard, buildProjectNavItem } from '$lib/workspace/presenters';

export const load: LayoutServerLoad = async ({ fetch }) => {
	try {
		const projects = await listProjects(fetch);
		const results = await Promise.allSettled(
			projects.map((project) => getProjectWorkspace(fetch, project.id))
		);
		const snapshots = results
			.filter(
				(
					result
				): result is PromiseFulfilledResult<Awaited<ReturnType<typeof getProjectWorkspace>>> =>
					result.status === 'fulfilled'
			)
			.map((result) => result.value);
		const failedCount = results.length - snapshots.length;
		const apiError = failedCount > 0 ? `${failedCount} project workspace request(s) failed.` : null;

		return {
			apiError,
			projectCards: snapshots.map(buildProjectCard),
			projectNav: snapshots.map(buildProjectNavItem)
		};
	} catch (error) {
		const message = error instanceof Error ? error.message : 'Workspace API is unavailable';

		return {
			apiError: message,
			projectCards: [],
			projectNav: []
		};
	}
};
