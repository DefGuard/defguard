import { useCallback, useMemo } from 'react';
import './style.scss';
import { Chip } from '../../defguard-ui/components/Chip/Chip';
import { FieldError } from '../../defguard-ui/components/FieldError/FieldError';
import { Fold } from '../../defguard-ui/components/Fold/Fold';
import { Icon } from '../../defguard-ui/components/Icon';
import { SizedBox } from '../../defguard-ui/components/SizedBox/SizedBox';
import { Toggle } from '../../defguard-ui/components/Toggle/Toggle';
import { ThemeSpacing, ThemeVariable } from '../../defguard-ui/types';
import { isPresent } from '../../defguard-ui/utils/isPresent';
import { useSelectionModal } from '../modals/SelectionModal/useSelectionModal';
import type {
  SelectionKey,
  SelectionSectionCustomRender,
} from '../SelectionSection/type';
import type { SelectMultipleProps } from './types';

export const SelectMultiple = <T extends number | string, M = unknown>({
  counterText,
  editText,
  editIcon,
  modalTitle,
  toggleText,
  options,
  selected,
  error,
  toggleValue,
  onSelectionChange,
  onToggleChange,
  selectionCustomItemRender,
  selectionModalProps,
}: SelectMultipleProps<T, M>) => {
  const selectedOptions = useMemo(
    () => options.filter((o) => selected.has(o.id)),
    [options, selected],
  );

  const handleSelectionSubmit = useCallback(
    (v: T[]) => {
      onSelectionChange(v);
    },
    [onSelectionChange],
  );

  const handleEdit = () => {
    useSelectionModal.setState({
      isOpen: true,
      contentClassName: selectionModalProps?.contentClassName,
      title: modalTitle,
      options,
      enableDividers: selectionModalProps?.enableDividers,
      itemGap: selectionModalProps?.itemGap,
      renderItem: selectionCustomItemRender as
        | SelectionSectionCustomRender<SelectionKey, unknown>
        | undefined,
      searchPlaceholder: selectionModalProps?.searchPlaceholder,
      //@ts-expect-error
      selected: selected,
      visibleItemsLimit: selectionModalProps?.visibleItemsLimit,
      //@ts-expect-error
      onSubmit: handleSelectionSubmit,
    });
  };

  return (
    <div className="select-multiple">
      {isPresent(toggleText) && (
        <Toggle
          disabled={options.length === 0}
          label={toggleText}
          active={toggleValue}
          onClick={() => {
            onToggleChange(!toggleValue);
          }}
        />
      )}
      <Fold open={!toggleValue}>
        {isPresent(toggleText) && <SizedBox height={ThemeSpacing.Xl} />}
        <div className="selected">
          {selectedOptions.map((o) => (
            <Chip text={o.label} key={o.id} />
          ))}
          {selectedOptions.length > 5 && <Chip text={counterText(selected.size - 5)} />}
        </div>
        {selectedOptions.length > 0 && <SizedBox height={ThemeSpacing.Md} />}
        <button type="button" onClick={handleEdit} className="select-multiple-edit">
          {isPresent(editIcon) && (
            <Icon icon={editIcon} size={20} staticColor={ThemeVariable.FgAction} />
          )}
          {editText}
        </button>
      </Fold>
      <FieldError error={error} />
    </div>
  );
};
