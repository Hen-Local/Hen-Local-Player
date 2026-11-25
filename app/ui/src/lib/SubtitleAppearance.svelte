<script lang="ts">
  import { player as p } from './stores.svelte'
  import { subtitleAppearanceDefaults } from './defaults'
  import Icon from './Icon.svelte'

  const controls = [
    { key: 'subtitleScale', label: 'scale', min: 0.6, max: 2, step: 0.1 },
    { key: 'subtitleBottomOffset', label: 'offset', min: 0, max: 0.4, step: 0.01 },
    { key: 'subtitleBackground', label: 'background', min: 0, max: 0.8, step: 0.05 },
  ] as const
</script>

<div class="subtitle-appearance">
  {#each controls as control}
    {@const value = p.settings[control.key]}
    {@const percent = Math.round(value * 100) + '%'}
    {@const span = control.max - control.min}
    {@const original = subtitleAppearanceDefaults[control.key]}
    {@const reset = p.t('resetAppearanceValue')
      .replace('{setting}', p.t(control.label))
      .replace('{value}', Math.round(original * 100) + '%')}
    <div class="appearance-row">
      <label class="slider-setting">
        <span>{p.t(control.label)}<b>{percent}</b></span>
        <span class="slider-rail" style:--default-position={100 * (original - control.min) / span + '%'}>
          <input
            type="range"
            min={control.min}
            max={control.max}
            step={control.step}
            value={value}
            style:--range-fill={100 * (value - control.min) / span + '%'}
            aria-label={p.t(control.label)}
            aria-valuetext={percent}
            oninput={(e) => { p.settings[control.key] = e.currentTarget.valueAsNumber }}
            onchange={(e) => {
              const value = e.currentTarget.valueAsNumber
              void p.act(() => p.update({ [control.key]: value }))
            }}
          />
        </span>
      </label>
      <button type="button" class="icon reset-value" aria-label={reset} title={reset}
        disabled={Math.fround(value) === Math.fround(original)}
        onclick={() => p.act(() => p.update({ [control.key]: original }))}>
        <Icon name="refresh" size={14} />
      </button>
    </div>
  {/each}
</div>

<style>
  .appearance-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    column-gap: 6px;
    align-items: end;
  }
  .slider-rail { position: relative; }
  .slider-rail::after {
    content: '';
    position: absolute;
    inset: auto 7px -1px;
    height: 3px;
    background: linear-gradient(currentColor, currentColor) var(--default-position) / 2px 100% no-repeat;
    opacity: 0.7;
    pointer-events: none;
  }
  .reset-value { width: 26px; height: 26px; min-height: 26px; padding: 0; }
</style>
