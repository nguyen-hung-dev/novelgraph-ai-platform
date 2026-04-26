import { error } from '@sveltejs/kit';
import { findProjectById } from '$lib/workspace/demo';

export function load({ params }: { params: { projectId: string } }) {
	const project = findProjectById(params.projectId);

	if (!project) {
		error(404, 'Project not found');
	}

	return {
		project
	};
}
