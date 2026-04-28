import { fail, redirect, type Actions } from '@sveltejs/kit';
import {
	activateManagedLocalModel,
	ApiClientError,
	checkByokKey as checkByokKeyHealth,
	downloadPresetLocalModel,
	getByokConfig,
	getLocalLlmHealth,
	getLocalLlmRuntime,
	listByokProviders,
	pickExistingLocalModel,
	saveByokConfig,
	startSelectedLocalModel,
	stopLocalLlmServer
} from '$lib/server/api';
import type { PageServerLoad } from './$types';

function actionErrorMessage(error: unknown, fallback: string) {
	if (error instanceof ApiClientError) {
		return error.message;
	}

	if (error instanceof Error) {
		return error.message;
	}

	return fallback;
}

export const load: PageServerLoad = async ({ fetch }) => {
	const [runtimeResult, healthResult, byokProvidersResult, byokConfigResult] = await Promise.allSettled([
		getLocalLlmRuntime(fetch),
		getLocalLlmHealth(fetch),
		listByokProviders(fetch),
		getByokConfig(fetch)
	]);

	return {
		runtime: runtimeResult.status === 'fulfilled' ? runtimeResult.value : null,
		runtimeError:
			runtimeResult.status === 'rejected'
				? runtimeResult.reason instanceof ApiClientError
					? runtimeResult.reason.message
					: 'Không thể nạp local LLM runtime.'
				: null,
		health: healthResult.status === 'fulfilled' ? healthResult.value : null,
		healthError:
			healthResult.status === 'rejected'
				? healthResult.reason instanceof ApiClientError
					? healthResult.reason.message
					: 'Không thể kiểm tra local LLM health.'
				: null,
		byokProviders: byokProvidersResult.status === 'fulfilled' ? byokProvidersResult.value : [],
		byokConfig: byokConfigResult.status === 'fulfilled' ? byokConfigResult.value : null,
		byokError:
			byokConfigResult.status === 'rejected'
				? byokConfigResult.reason instanceof ApiClientError
					? byokConfigResult.reason.message
					: 'Không thể nạp cấu hình BYOK.'
				: null
	};
};

export const actions: Actions = {
	saveByokSettings: async ({ fetch, request }) => {
		const formData = await request.formData();
		const provider = String(formData.get('provider') ?? '').trim();
		const baseUrl = String(formData.get('base_url') ?? '').trim();
		const model = String(formData.get('model') ?? '').trim();
		const apiKey = String(formData.get('api_key') ?? '').trim();

		try {
			const result = await saveByokConfig(fetch, {
				provider,
				base_url: baseUrl,
				model,
				api_key: apiKey || null,
				session_only: false
			});

			return {
				byokAction: {
					kind: 'saveByokSettings',
					success: true,
					message: result.saved_api_key
						? 'Đã lưu cấu hình và API key vào DB.'
						: 'Đã lưu cấu hình, giữ nguyên API key hiện có.'
				}
			};
		} catch (error) {
			return fail(400, {
				byokAction: {
					kind: 'saveByokSettings',
					success: false,
					error: actionErrorMessage(error, 'Không thể lưu cấu hình BYOK.')
				}
			});
		}
	},
	checkByokKey: async ({ fetch, request }) => {
		const formData = await request.formData();
		const provider = String(formData.get('provider') ?? '').trim();
		const baseUrl = String(formData.get('base_url') ?? '').trim();
		const model = String(formData.get('model') ?? '').trim();
		const apiKey = String(formData.get('api_key') ?? '').trim();

		try {
			const health = await checkByokKeyHealth(fetch, {
				provider,
				base_url: baseUrl,
				model,
				api_key: apiKey || null
			});

			return {
				byokAction: {
					kind: 'checkByokKey',
					success: health.valid,
					health,
					message: health.valid ? 'Healthy key.' : 'Key chưa healthy.'
				}
			};
		} catch (error) {
			return fail(400, {
				byokAction: {
					kind: 'checkByokKey',
					success: false,
					error: actionErrorMessage(error, 'Không thể kiểm tra API key.')
				}
			});
		}
	},
	pickExistingModel: async ({ fetch }) => {
		try {
			await pickExistingLocalModel(fetch);
		} catch (error) {
			return fail(400, {
				localRuntimeAction: {
					kind: 'pickExistingModel',
					error: actionErrorMessage(error, 'Không thể chọn file model lúc này.')
				}
			});
		}

		redirect(303, '/settings');
	},
	startSelectedModel: async ({ fetch }) => {
		try {
			await startSelectedLocalModel(fetch);
		} catch (error) {
			return fail(400, {
				localRuntimeAction: {
					kind: 'startSelectedModel',
					error: actionErrorMessage(error, 'Không thể khởi động model đã chọn.')
				}
			});
		}

		redirect(303, '/settings');
	},
	stopLocalServer: async ({ fetch }) => {
		try {
			await stopLocalLlmServer(fetch);
		} catch (error) {
			return fail(400, {
				localRuntimeAction: {
					kind: 'stopLocalServer',
					error: actionErrorMessage(error, 'Không thể dừng local llama-server.')
				}
			});
		}

		redirect(303, '/settings');
	},
	downloadPreset: async ({ fetch, request }) => {
		const formData = await request.formData();
		const presetId = String(formData.get('preset_id') ?? '').trim();

		if (!presetId) {
			return fail(400, {
				localRuntimeAction: {
					kind: 'downloadPreset',
					error: 'Thiếu preset id.',
					target: presetId
				}
			});
		}

		try {
			await downloadPresetLocalModel(fetch, presetId);
		} catch (error) {
			return fail(400, {
				localRuntimeAction: {
					kind: 'downloadPreset',
					error: actionErrorMessage(error, 'Không thể tải preset model lúc này.'),
					target: presetId
				}
			});
		}

		redirect(303, '/settings');
	},
	activateManagedModel: async ({ fetch, request }) => {
		const formData = await request.formData();
		const path = String(formData.get('path') ?? '').trim();

		if (!path) {
			return fail(400, {
				localRuntimeAction: {
					kind: 'activateManagedModel',
					error: 'Thiếu đường dẫn model.',
					target: path
				}
			});
		}

		try {
			await activateManagedLocalModel(fetch, { path });
		} catch (error) {
			return fail(400, {
				localRuntimeAction: {
					kind: 'activateManagedModel',
					error: actionErrorMessage(error, 'Không thể chạy model trong thư mục repo.'),
					target: path
				}
			});
		}

		redirect(303, '/settings');
	}
};
