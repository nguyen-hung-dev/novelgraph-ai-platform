export type AnalysisExecutionProfile = 'local_small_staged' | 'cloud_gemini_one_shot';

export type ChapterRunRange = {
	from_chapter_num: number;
	to_chapter_num: number;
};

export const ANALYSIS_PROFILE_OPTIONS: Array<{
	value: AnalysisExecutionProfile;
	label: string;
}> = [
	{ value: 'local_small_staged', label: 'Local small staged' },
	{ value: 'cloud_gemini_one_shot', label: 'Gemini cloud one-shot' }
];

export const ANALYSIS_COPY = {
	metrics: {
		progress: {
			label: 'Progress',
			detail: 'Chương completed trong analysis job hiện tại'
		},
		nextChapter: {
			label: 'Next chapter',
			detail: 'Resume sẽ chạy từ chương này và bỏ qua các chương completed',
			none: 'None'
		},
		chapterStates: {
			label: 'Chapter states'
		},
		status: {
			label: 'Status',
			fallbackDetail: 'Import truyện trước',
			idle: 'Idle'
		}
	},
	runner: {
		title: 'Analysis runner',
		subtitle: 'Điều khiển analysis theo từng chương với profile local hoặc Gemini cloud',
		progressAria: 'Analysis progress',
		createdLabel: 'Created',
		updatedLabel: 'Updated',
		uiStatusPrefix: 'UI',
		runtimeNoteTitle: 'Runtime note',
		cancelFailedTitle: 'Cancel job failed',
		cancelAcceptedTitle: 'Cancel request accepted',
		cancelAcceptedMeta: 'Trang sẽ nạp lại trạng thái mới sau request.',
		fromChapterLabel: 'Từ chương',
		toChapterLabel: 'Đến chương',
		profileLabel: 'Analysis profile',
		resumeButton: 'Resume',
		startButton: 'Start / chạy tiếp',
		pauseButton: 'Pause',
		forceButton: 'Force rerun',
		cancelButton: 'Cancel job',
		settingsLink: 'Settings / BYOK',
		emptyNote:
			'Chưa có analysis job nào cho project này. Xác nhận import truyện sẽ tạo pending job đầu tiên.'
	},
	telemetry: {
		title: 'Latest chapter telemetry',
		profile: 'Profile',
		status: 'status',
		calls: 'calls',
		provider: 'Provider',
		model: 'model',
		tokens: 'tokens',
		empty: 'n/a'
	},
	runPolicy: {
		title: 'Run policy',
		body:
			'Nếu Từ chương và Đến chương giống nhau, runner chỉ chạy chương đó. Nếu Đến chương lớn hơn Từ chương, runner chạy lần lượt trong phạm vi đã chọn và bỏ qua chương đã completed. Force rerun chỉ xóa trạng thái chapter run cũ trong phạm vi này. Profile local dùng llama.cpp. Profile cloud dùng BYOK Gemini key từ backend và mặc định chạy one-shot một call/chương, chỉ tăng call khi cần repair.'
	},
	errors: {
		missingJob: 'Chưa có analysis job để chạy.',
		invalidRangeInteger: 'Khoảng chương phải là số nguyên.',
		invalidRangeStart: 'Khoảng chương phải bắt đầu từ 1 trở lên.',
		invalidRangeOrder: 'Chương kết thúc phải lớn hơn hoặc bằng chương bắt đầu.',
		invalidRangeFallback: 'Khoảng chương không hợp lệ.',
		autoPaused: 'Analysis đã tự tạm dừng.',
		backendLost: 'Tự tạm dừng vì mất kết nối backend.'
	},
	notes: {
		jobCompleted: 'Đã chạy xong toàn bộ chương trong job hiện tại.',
		pauseQueued:
			'Đã nhận lệnh Pause. Request AI của chương hiện tại chưa bị cắt ngang; runner sẽ dừng sau khi request này trả về.',
		pausedAfterChapter: 'Đã tạm dừng sau chương hiện tại.',
		pauseWriteFailed:
			'Đã dừng vòng chạy UI, nhưng chưa ghi được trạng thái pause lên backend.'
	}
} as const;

export function analysisProfileLabel(profile: AnalysisExecutionProfile) {
	return (
		ANALYSIS_PROFILE_OPTIONS.find((option) => option.value === profile)?.label ??
		ANALYSIS_PROFILE_OPTIONS[0].label
	);
}

export function analysisRangeLabel(range: ChapterRunRange) {
	return range.from_chapter_num === range.to_chapter_num
		? `chương ${range.from_chapter_num}`
		: `chương ${range.from_chapter_num} -> ${range.to_chapter_num}`;
}

export function analysisChapterStatesDetail(pendingCount: number, failedCount: number) {
	return `${pendingCount} pending · ${failedCount} failed`;
}

export function analysisRunStartedNote(
	force: boolean,
	range: ChapterRunRange,
	profile: AnalysisExecutionProfile
) {
	const rangeText = analysisRangeLabel(range);
	const profileText = analysisProfileLabel(profile);
	return force
		? `Đang chạy lại ${rangeText} theo profile ${profileText} và ghi đè trạng thái cũ trong phạm vi này.`
		: `Đang chạy ${rangeText} theo profile ${profileText}.`;
}

export function analysisRequestFailedMessage(status: number) {
	return `Analysis request failed with HTTP ${status}`;
}

export function analysisLostConnectionNote(message?: string) {
	return message
		? `Tự tạm dừng vì mất kết nối hoặc request lỗi: ${message}`
		: ANALYSIS_COPY.errors.backendLost;
}

export function analysisForceRerunConfirm(range: ChapterRunRange) {
	return `Chạy lại ${analysisRangeLabel(range)} sẽ xóa trạng thái chapter run cũ trong phạm vi này.`;
}
