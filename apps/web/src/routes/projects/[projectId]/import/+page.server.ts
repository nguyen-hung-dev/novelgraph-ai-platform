import { fail, redirect, type Actions } from '@sveltejs/kit';
import { ApiClientError, confirmNovelImport, previewNovelImport } from '$lib/server/api';
import type { NovelImportInput } from '$lib/api/types';

async function readImportInput(formData: FormData): Promise<NovelImportInput> {
	const rawTitle = String(formData.get('title') ?? '').trim();
	const author = String(formData.get('author') ?? '').trim();
	const sourceLanguage = String(formData.get('source_language') ?? '').trim();
	const rawText = String(formData.get('text') ?? '');
	const maybeFile = formData.get('file');
	let text = rawText;
	let title = rawTitle;

	if (maybeFile instanceof File && maybeFile.size > 0) {
		text = await maybeFile.text();
		if (!title) {
			title = maybeFile.name.replace(/\.[^.]+$/, '');
		}
	}

	return {
		title,
		author: author || null,
		source_language: sourceLanguage || null,
		text
	};
}

function formState(input: NovelImportInput) {
	return {
		author: input.author ?? '',
		source_language: input.source_language ?? '',
		text: input.text,
		title: input.title
	};
}

export const actions: Actions = {
	preview: async ({ fetch, params, request }) => {
		const projectId = params.projectId;
		if (!projectId) {
			return fail(404, {
				importForm: {
					author: '',
					error: 'Không tìm thấy project.',
					source_language: '',
					text: '',
					title: ''
				}
			});
		}

		const input = await readImportInput(await request.formData());

		if (!input.title.trim()) {
			return fail(400, {
				importForm: {
					...formState(input),
					error: 'Novel title is required.'
				}
			});
		}

		if (!input.text.trim()) {
			return fail(400, {
				importForm: {
					...formState(input),
					error: 'Novel text is required. Upload a file or paste the source text.'
				}
			});
		}

		try {
			const preview = await previewNovelImport(fetch, projectId, input);
			return {
				importForm: formState(input),
				preview
			};
		} catch (error) {
			const message = error instanceof ApiClientError ? error.message : 'Preview request failed.';

			return fail(400, {
				importForm: {
					...formState(input),
					error: message
				}
			});
		}
	},
	confirm: async ({ fetch, params, request }) => {
		const projectId = params.projectId;
		if (!projectId) {
			return fail(404, {
				importForm: {
					author: '',
					error: 'Không tìm thấy project.',
					source_language: '',
					text: '',
					title: ''
				}
			});
		}

		const input = await readImportInput(await request.formData());

		if (!input.title.trim()) {
			return fail(400, {
				importForm: {
					...formState(input),
					error: 'Novel title is required.'
				}
			});
		}

		if (!input.text.trim()) {
			return fail(400, {
				importForm: {
					...formState(input),
					error: 'Novel text is required. Upload a file or paste the source text.'
				}
			});
		}

		try {
			await confirmNovelImport(fetch, projectId, input);
		} catch (error) {
			const message = error instanceof ApiClientError ? error.message : 'Confirm import failed.';

			return fail(400, {
				importForm: {
					...formState(input),
					error: message
				}
			});
		}

		redirect(303, `/projects/${projectId}/reading`);
	}
};
