import { env } from '$env/dynamic/private';
import type { RequestHandler } from './$types';

const hopByHopHeaders = new Set([
	'connection',
	'content-encoding',
	'content-length',
	'keep-alive',
	'proxy-authenticate',
	'proxy-authorization',
	'te',
	'trailer',
	'transfer-encoding',
	'upgrade'
]);

const proxy: RequestHandler = async ({ request, params, url }) => {
	const backendBase = (env.BACKEND_INTERNAL_URL || 'http://127.0.0.1:8080').replace(/\/$/, '');
	const target = new URL(`/api/${params.path ?? ''}${url.search}`, backendBase);
	const headers = new Headers(request.headers);
	headers.delete('host');

	const response = await fetch(target, {
		method: request.method,
		headers,
		body: request.method === 'GET' || request.method === 'HEAD' ? undefined : await request.arrayBuffer(),
		redirect: 'manual'
	});

	const responseHeaders = new Headers(response.headers);
	for (const header of hopByHopHeaders) {
		responseHeaders.delete(header);
	}

	return new Response(response.body, {
		status: response.status,
		statusText: response.statusText,
		headers: responseHeaders
	});
};

export const GET = proxy;
export const POST = proxy;
export const PUT = proxy;
export const PATCH = proxy;
export const DELETE = proxy;
