import { fail, redirect, type Actions } from '@sveltejs/kit';
import {
	aiFillNovelMetadata,
	ApiClientError,
	confirmNovelImport,
	previewNovelImport,
	suggestNovelImportMetadata,
	updateNovelMetadata
} from '$lib/server/api';
import type { NovelImportInput, NovelMetadataUpdateInput } from '$lib/api/types';

async function readImportInput(formData: FormData): Promise<NovelImportInput> {
	const rawTitle = String(formData.get('title') ?? '').trim();
	const author = String(formData.get('author') ?? '').trim();
	const sourceLanguage = String(formData.get('source_language') ?? '').trim();
	const genre = String(formData.get('genre') ?? '').trim();
	const description = String(formData.get('description') ?? '').trim();
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
		source_language: sourceLanguage && sourceLanguage !== 'auto' ? sourceLanguage : null,
		genre: genre || null,
		description: description || null,
		text
	};
}

function formState(input: NovelImportInput) {
	return {
		author: input.author ?? '',
		description: input.description ?? '',
		genre: input.genre ?? '',
		source_language: input.source_language ?? '',
		text: input.text,
		title: input.title
	};
}

function emptyImportForm(error: string) {
	return {
		author: '',
		description: '',
		error,
		genre: '',
		source_language: '',
		text: '',
		title: ''
	};
}

function readNovelMetadataInput(formData: FormData): NovelMetadataUpdateInput {
	const sourceLanguage = String(formData.get('metadata_source_language') ?? '').trim();

	return {
		title: String(formData.get('metadata_title') ?? '').trim(),
		author: String(formData.get('metadata_author') ?? '').trim() || null,
		source_language: sourceLanguage && sourceLanguage !== 'auto' ? sourceLanguage : null,
		genre: String(formData.get('metadata_genre') ?? '').trim() || null,
		description: String(formData.get('metadata_description') ?? '').trim() || null
	};
}

export const actions: Actions = {
	preview: async ({ fetch, params, request }) => {
		const projectId = params.projectId;
		if (!projectId) {
			return fail(404, {
				importForm: emptyImportForm('Không tìm thấy project.')
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
				importForm: emptyImportForm('Không tìm thấy project.')
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
	},
	metadata: async ({ fetch, params, request }) => {
		const projectId = params.projectId;
		if (!projectId) {
			return fail(404, {
				importForm: emptyImportForm('Không tìm thấy project.')
			});
		}

		const input = await readImportInput(await request.formData());
		if (!input.text.trim()) {
			return fail(400, {
				importForm: {
					...formState(input),
					error: 'Novel text is required. Upload a file or paste the source text.'
				}
			});
		}

		try {
			const suggestion = await suggestNovelImportMetadata(fetch, projectId, input);
			return {
				importForm: {
					...formState({
						...input,
						title: suggestion.title ?? input.title,
						author: suggestion.author ?? input.author,
						source_language: suggestion.source_language ?? input.source_language,
						genre: suggestion.genre ?? input.genre,
						description: suggestion.description ?? input.description
					}),
					message: 'AI đã điền metadata từ văn bản nguồn.'
				}
			};
		} catch (error) {
			const message =
				error instanceof ApiClientError ? error.message : 'AI metadata request failed.';

			return fail(400, {
				importForm: {
					...formState(input),
					error: message
				}
			});
		}
	},
	saveMetadata: async ({ fetch, params, request }) => {
		const projectId = params.projectId;
		const formData = await request.formData();
		const novelId = String(formData.get('novel_id') ?? '').trim();
		if (!projectId || !novelId) {
			return fail(404, {
				metadataError: 'Không tìm thấy truyện.'
			});
		}

		const input = readNovelMetadataInput(formData);
		if (!input.title?.trim()) {
			return fail(400, {
				metadataError: 'Tên truyện là bắt buộc.'
			});
		}

		try {
			await updateNovelMetadata(fetch, projectId, novelId, input);
			return {
				metadataMessage: 'Đã lưu metadata truyện vào DB.'
			};
		} catch (error) {
			const message = error instanceof ApiClientError ? error.message : 'Metadata save failed.';

			return fail(400, {
				metadataError: message
			});
		}
	},
	aiFillActiveMetadata: async ({ fetch, params, request }) => {
		const projectId = params.projectId;
		const formData = await request.formData();
		const novelId = String(formData.get('novel_id') ?? '').trim();
		if (!projectId || !novelId) {
			return fail(404, {
				metadataError: 'Không tìm thấy truyện.'
			});
		}

		try {
			await aiFillNovelMetadata(fetch, projectId, novelId);
			return {
				metadataMessage: 'AI đã điền metadata và ghi vào DB.'
			};
		} catch (error) {
			const message =
				error instanceof ApiClientError ? error.message : 'AI metadata request failed.';

			return fail(400, {
				metadataError: message
			});
		}
	}
};
