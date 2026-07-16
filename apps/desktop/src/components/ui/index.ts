import type { App } from 'vue';

import UiAlert from './UiAlert.vue';
import UiButton from './UiButton.vue';
import UiCard from './UiCard.vue';
import UiDateTimePicker from './UiDateTimePicker.vue';
import UiDialog from './UiDialog.vue';
import UiForm from './UiForm.vue';
import UiFormItem from './UiFormItem.vue';
import UiInput from './UiInput.vue';
import UiInputNumber from './UiInputNumber.vue';
import UiOption from './UiOption.vue';
import UiPagination from './UiPagination.vue';
import UiSelect from './UiSelect.vue';
import UiSwitch from './UiSwitch.vue';
import UiTable from './UiTable.vue';
import UiTableColumn from './UiTableColumn.vue';
import UiTag from './UiTag.vue';

export const UiComponents = {
  install(app: App) {
    app.component('UiAlert', UiAlert);
    app.component('UiButton', UiButton);
    app.component('UiCard', UiCard);
    app.component('UiDateTimePicker', UiDateTimePicker);
    app.component('UiDialog', UiDialog);
    app.component('UiForm', UiForm);
    app.component('UiFormItem', UiFormItem);
    app.component('UiInput', UiInput);
    app.component('UiInputNumber', UiInputNumber);
    app.component('UiOption', UiOption);
    app.component('UiPagination', UiPagination);
    app.component('UiSelect', UiSelect);
    app.component('UiSwitch', UiSwitch);
    app.component('UiTable', UiTable);
    app.component('UiTableColumn', UiTableColumn);
    app.component('UiTag', UiTag);
  },
};
