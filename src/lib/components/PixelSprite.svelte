<script lang="ts">
  import type { AssetManifestEntry } from '../types/atlas';

  interface Props {
    asset?: AssetManifestEntry;
    fallbackUrl?: string;
    size?: number;
    alt?: string;
  }

  let { asset, fallbackUrl = '/assets-fallback/item-fallback.svg', size = 24, alt = '' }: Props = $props();
  let broken = $state(false);

  $effect(() => {
    // Reset broken state when asset url changes
    asset?.url;
    broken = false;
  });

  const src = $derived(broken || !asset ? fallbackUrl : asset.url);
</script>

<img
  {src}
  alt={alt || asset?.nameRu || ''}
  width={size}
  height={size}
  loading="lazy"
  decoding="async"
  onerror={() => {
    broken = true;
  }}
  class="pixel-sprite"
  style={`width:${size}px;height:${size}px;object-fit:${asset?.animatedStrip ? 'cover' : 'contain'};object-position:top center;`}
/>

<style>
  .pixel-sprite {
    display: inline-block;
    flex-shrink: 0;
    image-rendering: pixelated;
    image-rendering: crisp-edges;
    user-select: none;
    pointer-events: none;
  }
</style>
