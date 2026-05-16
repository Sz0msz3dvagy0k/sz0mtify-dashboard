<script lang="ts">
	import type { QueueTrack } from '$lib/player';
	import { swipeQueue } from '$lib/swipeQueue';

	export let track: QueueTrack | null | undefined;
</script>

<div class="swipe-queue-handle" use:swipeQueue={{ track }}>
	<slot />
</div>
