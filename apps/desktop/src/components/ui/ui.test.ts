import { mount } from '@vue/test-utils';
import { nextTick, ref } from 'vue';
import { describe, expect, it } from 'vitest';

import UiButton from './UiButton.vue';
import UiDateTimePicker from './UiDateTimePicker.vue';
import UiDialog from './UiDialog.vue';
import UiInput from './UiInput.vue';
import UiSelect from './UiSelect.vue';
import UiTable from './UiTable.vue';
import UiTableColumn from './UiTableColumn.vue';
import UiTag from './UiTag.vue';

describe('ui primitives', () => {
  it('emits clicks from UiButton', async () => {
    const wrapper = mount(UiButton, {
      slots: {
        default: 'Save',
      },
    });

    await wrapper.get('button').trigger('click');

    expect(wrapper.emitted('click')).toHaveLength(1);
  });

  it('uses neutral styling for primary link buttons', () => {
    const wrapper = mount(UiButton, {
      props: {
        link: true,
        type: 'primary',
      },
      slots: {
        default: 'Edit',
      },
    });

    const classes = wrapper.get('button').classes();
    expect(classes).toContain('text-zinc-900');
    expect(classes.some((item) => item.startsWith('text-blue-'))).toBe(false);
  });

  it('updates UiInput model value', async () => {
    const wrapper = mount(UiInput, {
      props: {
        modelValue: '',
      },
    });

    await wrapper.get('input').setValue('admin');

    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['admin']);
  });

  it('reads and emits UTC timestamps from UiDateTimePicker', async () => {
    const wrapper = mount(UiDateTimePicker, {
      props: {
        modelValue: '2026-07-16T14:00:00Z',
        label: 'Start time (UTC)',
      },
      attrs: {
        class: 'audit-time-filter',
        name: 'startedAt',
        step: '60',
      },
    });

    expect(wrapper.classes()).toContain('audit-time-filter');
    expect((wrapper.get('input').element as HTMLInputElement).value).toBe('2026-07-16T14:00');
    await wrapper.get('input').setValue('2026-07-16T15:30');

    expect(wrapper.get('input').attributes('type')).toBe('datetime-local');
    expect(wrapper.get('input').attributes('name')).toBe('startedAt');
    expect(wrapper.get('input').attributes('step')).toBe('60');
    expect(wrapper.get('input').attributes('aria-label')).toBe('Start time (UTC)');
    expect(wrapper.text()).toContain('UTC');
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['2026-07-16T15:30:00.000Z']);
  });

  it('forwards updated attributes to the date-time input', async () => {
    const name = ref('startedAt');
    const wrapper = mount({
      components: { UiDateTimePicker },
      setup: () => ({ name }),
      template: '<UiDateTimePicker model-value="" :name="name" />',
    });

    expect(wrapper.get('input').attributes('name')).toBe('startedAt');
    name.value = 'endedAt';
    await nextTick();
    expect(wrapper.get('input').attributes('name')).toBe('endedAt');
  });

  it('updates UiSelect model value from the application menu', async () => {
    const wrapper = mount(UiSelect, {
      props: {
        modelValue: '',
      },
      slots: {
        default: '<UiOption label="Enabled" value="enabled" />',
      },
      global: {
        components: {
          UiOption: (await import('./UiOption.vue')).default,
        },
      },
    });

    expect(wrapper.find('select').exists()).toBe(false);

    await wrapper.get('[data-test="ui-select-trigger"]').trigger('click');
    const option = document.body.querySelector('[data-test="ui-select-option-0"]') as HTMLElement;
    option.click();
    await nextTick();

    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['enabled']);
    wrapper.unmount();
  });

  it('clears UiSelect model value from the application menu', async () => {
    const wrapper = mount(UiSelect, {
      props: {
        modelValue: 'enabled',
        clearable: true,
        placeholder: 'Method',
      },
      slots: {
        default: '<UiOption label="Enabled" value="enabled" />',
      },
      global: {
        components: {
          UiOption: (await import('./UiOption.vue')).default,
        },
      },
    });

    await wrapper.get('[data-test="ui-select-trigger"]').trigger('click');
    const clearOption = document.body.querySelector('[data-test="ui-select-clear"]') as HTMLElement;
    expect(clearOption.textContent).toContain('All Method');
    clearOption.click();
    await nextTick();

    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual([undefined]);
    wrapper.unmount();
  });

  it('updates UiSelect multiple values without closing the menu', async () => {
    const wrapper = mount(UiSelect, {
      props: {
        modelValue: [],
        multiple: true,
      },
      slots: {
        default: `
          <UiOption label="Admin" value="admin" />
          <UiOption label="Editor" value="editor" />
        `,
      },
      global: {
        components: {
          UiOption: (await import('./UiOption.vue')).default,
        },
      },
    });

    await wrapper.get('[data-test="ui-select-trigger"]').trigger('click');
    (document.body.querySelector('[data-test="ui-select-option-0"]') as HTMLElement).click();
    await nextTick();

    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual([['admin']]);
    expect(document.body.querySelector('[data-test="ui-select-menu"]')).not.toBeNull();

    await wrapper.setProps({ modelValue: ['admin'] });
    (document.body.querySelector('[data-test="ui-select-option-1"]') as HTMLElement).click();
    await nextTick();

    expect(wrapper.emitted('update:modelValue')?.[1]).toEqual([['admin', 'editor']]);

    await wrapper.setProps({ modelValue: ['admin', 'editor'] });
    (document.body.querySelector('[data-test="ui-select-option-0"]') as HTMLElement).click();
    await nextTick();

    expect(wrapper.emitted('update:modelValue')?.[2]).toEqual([['editor']]);
    wrapper.unmount();
  });

  it('renders UiSelect menu as a fixed overlay outside clipped parents', async () => {
    const wrapper = mount(UiDialog, {
      attachTo: document.body,
      props: {
        modelValue: true,
        title: 'Assign roles',
      },
      slots: {
        default: `
          <div style="overflow: hidden">
            <UiSelect model-value="" placeholder="Select roles">
              <UiOption label="Admin" value="admin" />
            </UiSelect>
          </div>
        `,
      },
      global: {
        components: {
          UiSelect,
          UiOption: (await import('./UiOption.vue')).default,
        },
      },
    });

    await wrapper.get('[data-test="ui-select-trigger"]').trigger('click');
    const menu = document.body.querySelector('[data-test="ui-select-menu"]') as HTMLElement;

    expect(menu).not.toBeNull();
    expect(menu.style.position).toBe('fixed');

    wrapper.unmount();
  });

  it('limits UiSelect menu height to available viewport space', async () => {
    const originalInnerHeight = window.innerHeight;
    Object.defineProperty(window, 'innerHeight', { configurable: true, value: 300 });
    const wrapper = mount(UiSelect, {
      props: {
        modelValue: '',
      },
      slots: {
        default: `
          <UiOption label="One" value="one" />
          <UiOption label="Two" value="two" />
          <UiOption label="Three" value="three" />
          <UiOption label="Four" value="four" />
          <UiOption label="Five" value="five" />
          <UiOption label="Six" value="six" />
        `,
      },
      global: {
        components: {
          UiOption: (await import('./UiOption.vue')).default,
        },
      },
    });
    const trigger = wrapper.get('[data-test="ui-select-trigger"]').element as HTMLElement;
    trigger.getBoundingClientRect = () =>
      ({
        width: 220,
        height: 42,
        top: 80,
        right: 228,
        bottom: 122,
        left: 8,
        x: 8,
        y: 80,
        toJSON: () => ({}),
      }) as DOMRect;

    await wrapper.get('[data-test="ui-select-trigger"]').trigger('click');
    await nextTick();
    const menu = document.body.querySelector('[data-test="ui-select-menu"]') as HTMLElement;

    expect(menu.style.maxHeight).toBe('166px');

    wrapper.unmount();
    Object.defineProperty(window, 'innerHeight', { configurable: true, value: originalInnerHeight });
  });

  it('renders UiDialog only when open', () => {
    const wrapper = mount(UiDialog, {
      props: {
        modelValue: true,
        title: 'EditUser',
      },
      slots: {
        default: 'Form content',
      },
    });

    expect(wrapper.text()).toContain('EditUser');
    expect(wrapper.text()).toContain('Form content');
  });

  it('renders UiTable columns and scoped cells', async () => {
    const wrapper = mount(UiTable, {
      props: {
        data: [{ name: 'admin', enable: 1 }],
      },
      slots: {
        default: `
          <UiTableColumn prop="name" label="Username" />
          <UiTableColumn label="Status">
            <template #default="{ row }">
              <UiTag :type="row.enable === 1 ? 'success' : 'danger'">Enabled</UiTag>
            </template>
          </UiTableColumn>
        `,
      },
      global: {
        components: {
          UiTableColumn,
          UiTag,
        },
      },
    });

    await nextTick();

    expect(wrapper.text()).toContain('Username');
    expect(wrapper.text()).toContain('admin');
    expect(wrapper.text()).toContain('Enabled');
    expect(wrapper.find('.tree-cell').exists()).toBe(false);
    expect(wrapper.emitted('selection-change')).toBeUndefined();
  });

  it('keeps existing UiTable rows visible while refreshing', async () => {
    const wrapper = mount(UiTable, {
      props: {
        data: [{ name: 'admin' }],
        loading: true,
      },
      slots: {
        default: '<UiTableColumn prop="name" label="Username" />',
      },
      global: {
        components: {
          UiTableColumn,
        },
      },
    });

    await nextTick();

    expect(wrapper.text()).toContain('admin');
    expect(wrapper.text()).not.toContain('Loading');
  });

  it('collapses tree rows by default and expands them on demand', async () => {
    const wrapper = mount(UiTable, {
      props: {
        data: [
          {
            id: 1,
            name: 'Parent',
            children: [{ id: 2, name: 'Child' }],
          },
        ],
      },
      slots: {
        default: '<UiTableColumn prop="name" label="Name" />',
      },
      global: {
        components: {
          UiTableColumn,
        },
      },
    });

    await nextTick();

    expect(wrapper.text()).toContain('Parent');
    expect(wrapper.text()).not.toContain('Child');
    expect(wrapper.find('.tree-cell').exists()).toBe(true);
    expect(wrapper.get('[data-test="tree-toggle-1"]').find('svg.h-5.w-5').exists()).toBe(true);
    await wrapper.get('[data-test="tree-toggle-1"]').trigger('click');
    expect(wrapper.text()).toContain('Child');
    await wrapper.get('[data-test="tree-toggle-1"]').trigger('click');
    expect(wrapper.text()).not.toContain('Child');

    await wrapper.setProps({ defaultExpandAll: true });
    expect(wrapper.text()).toContain('Child');
  });
});
