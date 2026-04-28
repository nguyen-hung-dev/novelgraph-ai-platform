export type ProjectRealtimeEvent = {
	project_id: string;
	event_type: string;
	job_id: string | null;
	chapter_id: string | null;
	detail: string;
};

export function connectProjectRealtime(
	projectId: string,
	onEvent: (event: ProjectRealtimeEvent) => void
) {
	let socket: WebSocket | null = null;
	let reconnectTimer: number | null = null;
	let closed = false;

	function connect() {
		if (closed) {
			return;
		}

		socket = new WebSocket(projectRealtimeUrl(projectId));
		socket.onmessage = (message) => {
			const event = parseRealtimeEvent(message.data);
			if (event) {
				onEvent(event);
			}
		};
		socket.onerror = () => {
			socket?.close();
		};
		socket.onclose = () => {
			socket = null;
			if (!closed) {
				reconnectTimer = window.setTimeout(connect, 1500);
			}
		};
	}

	connect();

	return () => {
		closed = true;
		if (reconnectTimer !== null) {
			window.clearTimeout(reconnectTimer);
		}
		socket?.close();
	};
}

function projectRealtimeUrl(projectId: string) {
	const apiBase = import.meta.env.VITE_API_BASE_URL ?? 'http://127.0.0.1:3000';
	const url = new URL(apiBase, window.location.origin);
	url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
	url.pathname = `/api/projects/${encodeURIComponent(projectId)}/realtime`;
	url.search = '';
	url.hash = '';

	return url.toString();
}

function parseRealtimeEvent(data: unknown): ProjectRealtimeEvent | null {
	if (typeof data !== 'string') {
		return null;
	}

	try {
		return JSON.parse(data) as ProjectRealtimeEvent;
	} catch {
		return null;
	}
}
