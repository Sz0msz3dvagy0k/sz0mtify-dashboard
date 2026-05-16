import { browser } from '$app/environment';
import { queueTrackAtTop, type QueueTrack } from '$lib/player';

type SwipeQueueOptions = {
	track: QueueTrack | null | undefined;
	enabled?: boolean;
};

const swipeThreshold = 76;
const verticalTolerance = 52;
const maxPreviewOffset = 18;

export function swipeQueue(node: HTMLElement, options: SwipeQueueOptions) {
	let currentOptions = options;
	let startX = 0;
	let startY = 0;
	let tracking = false;
	let suppressNextClick = false;
	const media = browser ? window.matchMedia('(max-width: 980px)') : null;

	function isEnabled() {
		return Boolean(currentOptions.enabled ?? true) && Boolean(currentOptions.track) && Boolean(media?.matches);
	}

	function resetPreview() {
		tracking = false;
		node.classList.remove('swiping-queue');
		node.style.removeProperty('--swipe-progress');
	}

	function touchStart(event: TouchEvent) {
		if (!isEnabled() || event.touches.length !== 1) return;
		const target = event.target instanceof Element ? event.target : null;
		const interactive = target?.closest('button, a, input, select, textarea');
		if (interactive && interactive !== node) return;

		const touch = event.touches[0];
		startX = touch.clientX;
		startY = touch.clientY;
		tracking = true;
	}

	function touchMove(event: TouchEvent) {
		if (!tracking || event.touches.length !== 1) return;
		const touch = event.touches[0];
		const deltaX = touch.clientX - startX;
		const deltaY = touch.clientY - startY;
		if (deltaX <= 0 || Math.abs(deltaY) > Math.abs(deltaX)) {
			resetPreview();
			return;
		}

		node.classList.add('swiping-queue');
		node.style.setProperty('--swipe-progress', String(Math.min(deltaX / swipeThreshold, 1)));
		if (deltaX > 8 && event.cancelable) event.preventDefault();
	}

	function touchEnd(event: TouchEvent) {
		if (!tracking) return;
		const touch = event.changedTouches[0];
		const deltaX = touch.clientX - startX;
		const deltaY = touch.clientY - startY;
		const track = currentOptions.track;
		resetPreview();

		if (!track || deltaX < swipeThreshold || Math.abs(deltaY) > verticalTolerance || deltaX < Math.abs(deltaY) * 1.4) {
			return;
		}

		queueTrackAtTop(track);
		suppressNextClick = true;
		node.classList.add('queued-at-top');
		window.setTimeout(() => node.classList.remove('queued-at-top'), 420);
		if (event.cancelable) event.preventDefault();
	}

	function click(event: MouseEvent) {
		if (!suppressNextClick) return;
		suppressNextClick = false;
		event.preventDefault();
		event.stopImmediatePropagation();
	}

	node.classList.add('swipe-queue-target');
	node.addEventListener('touchstart', touchStart, { passive: true });
	node.addEventListener('touchmove', touchMove, { passive: false });
	node.addEventListener('touchend', touchEnd);
	node.addEventListener('touchcancel', resetPreview);
	node.addEventListener('click', click, true);

	return {
		update(nextOptions: SwipeQueueOptions) {
			currentOptions = nextOptions;
		},
		destroy() {
			node.classList.remove('swipe-queue-target', 'swiping-queue', 'queued-at-top');
			node.removeEventListener('touchstart', touchStart);
			node.removeEventListener('touchmove', touchMove);
			node.removeEventListener('touchend', touchEnd);
			node.removeEventListener('touchcancel', resetPreview);
			node.removeEventListener('click', click, true);
			node.style.removeProperty('--swipe-progress');
		}
	};
}
