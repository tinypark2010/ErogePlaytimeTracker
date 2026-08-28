<script lang="ts">
  import {
    daysInMonth,
    emptyDateTimeParts,
    formatDateTimeValue,
    parseDateTimeValue,
    updateDateTimePart,
  } from '../lib/dateTimeParts';
  import type { DateTimePart } from '../lib/dateTimeParts';

  export let value = '';
  export let label: string;
  export let withTime = true;
  export let optional = false;
  export let disabled = false;
  export let invalid = false;
  export let onchange: (value: string, complete: boolean) => void;

  const twoDigits = (value: number) => String(value).padStart(2, '0');
  const numberedOptions = (start: number, end: number) =>
    Array.from({ length: end - start + 1 }, (_, index) => twoDigits(start + index));
  const months = numberedOptions(1, 12);
  const hours = numberedOptions(0, 23);
  const minutes = numberedOptions(0, 59);
  const seconds = numberedOptions(0, 59);
  const currentYear = new Date().getFullYear();

  let parts = parseDateTimeValue(value, withTime);
  let synchronizedValue = value;
  let synchronizedWithTime = withTime;
  let years: string[];
  let days: string[];
  let hasSelection = false;

  $: if (value !== synchronizedValue || withTime !== synchronizedWithTime) {
    parts = parseDateTimeValue(value, withTime);
    synchronizedValue = value;
    synchronizedWithTime = withTime;
  }
  $: years = (() => {
    const options = Array.from({ length: currentYear + 5 - 1980 + 1 }, (_, index) =>
      String(currentYear + 5 - index),
    );
    return parts.year && !options.includes(parts.year)
      ? [...options, parts.year].sort((left, right) => Number(right) - Number(left))
      : options;
  })();
  $: days = numberedOptions(1, daysInMonth(parts.year, parts.month));
  $: hasSelection = Object.values(parts).some(Boolean);

  function changePart(part: DateTimePart, event: Event) {
    parts = updateDateTimePart(parts, part, (event.currentTarget as HTMLSelectElement).value);
    const nextValue = formatDateTimeValue(parts, withTime);
    const complete = Boolean(nextValue) || !Object.values(parts).some(Boolean);
    value = nextValue;
    synchronizedValue = nextValue;
    onchange(nextValue, complete);
  }

  function clear() {
    parts = emptyDateTimeParts();
    value = '';
    synchronizedValue = '';
    onchange('', true);
  }
</script>

<fieldset class:invalid class:date-only={!withTime} class="date-time-select" {disabled}>
  <legend>{label}{optional ? '（任意）' : ''}</legend>
  <div class="date-time-select-fields">
    <label
      ><span>年</span><select
        value={parts.year}
        aria-label={`${label}の年`}
        aria-invalid={invalid}
        onchange={(event) => changePart('year', event)}
        ><option value="">----</option>{#each years as year}<option value={year}>{year}</option
          >{/each}</select
      ></label
    ><label
      ><span>月</span><select
        value={parts.month}
        aria-label={`${label}の月`}
        aria-invalid={invalid}
        onchange={(event) => changePart('month', event)}
        ><option value="">--</option>{#each months as month}<option value={month}>{month}</option
          >{/each}</select
      ></label
    ><label
      ><span>日</span><select
        value={parts.day}
        aria-label={`${label}の日`}
        aria-invalid={invalid}
        onchange={(event) => changePart('day', event)}
        ><option value="">--</option>{#each days as day}<option value={day}>{day}</option
          >{/each}</select
      ></label
    >
    {#if withTime}<span class="date-time-separator" aria-hidden="true"></span><label
        ><span>時</span><select
          value={parts.hour}
          aria-label={`${label}の時`}
          aria-invalid={invalid}
          onchange={(event) => changePart('hour', event)}
          ><option value="">--</option>{#each hours as hour}<option value={hour}>{hour}</option
            >{/each}</select
        ></label
      ><label
        ><span>分</span><select
          value={parts.minute}
          aria-label={`${label}の分`}
          aria-invalid={invalid}
          onchange={(event) => changePart('minute', event)}
          ><option value="">--</option>{#each minutes as minute}<option value={minute}
              >{minute}</option
            >{/each}</select
        ></label
      ><label
        ><span>秒</span><select
          value={parts.second}
          aria-label={`${label}の秒`}
          aria-invalid={invalid}
          onchange={(event) => changePart('second', event)}
          ><option value="">--</option>{#each seconds as second}<option value={second}
              >{second}</option
            >{/each}</select
        ></label
      >{/if}
  </div>
  {#if optional}<button
      class="date-time-clear"
      type="button"
      disabled={disabled || !hasSelection}
      onclick={clear}>未設定にする</button
    >{/if}
</fieldset>
