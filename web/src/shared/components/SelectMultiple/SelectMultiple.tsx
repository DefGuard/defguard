import { useCallback, useMemo } from 'react';
import './style.scss';
import { Chip } from '../../defguard-ui/components/Chip/Chip';
import { FieldError } from '../../defguard-ui/components/FieldError/FieldError';
import { Fold } from '../../defguard-ui/components/Fold/Fold';
import { SizedBox } from '../../defguard-ui/components/SizedBox/SizedBox';
import { Toggle } from '../../defguard-ui/components/Toggle/Toggle';
import { ThemeSpacing } from '../../defguard-ui/types';
import { isPresent } from '../../defguard-ui/utils/isPresent';
import { useSelectionModal } from '../modals/SelectionModal/useSelectionModal';
import type { SelectMultipleProps } from './types';

export const SelectMultiple = <T extends number | string, M = unknown>({
  counterText,
  editText,
  modalTitle,
  toggleText,
  options,
  selected,
  error,
  toggleValue,
  onSelectionChange,
  onToggleChange,
}: SelectMultipleProps<T, M>) => {
  const selectedOptions = useMemo(
    () => options.filter((o) => selected.has(o.id)),
    [options, selected],
  );

  const handleSelectionCancel = useCallback(() => {
    if (selected.size === 0) {
      onToggleChange(true);
    }
  }, [onToggleChange, selected.size]);

  const handleSelectionSubmit = useCallback(
    (v: T[]) => {
      if (!v.length) {
        onToggleChange(true);
      }
      onSelectionChange(v);
    },
    [onToggleChange, onSelectionChange],
  );

  const handleEdit = () => {
    useSelectionModal.setState({
      isOpen: true,
      title: modalTitle,
      options,
      //@ts-expect-error
      selected: selected,
      //@ts-expect-error
      onSubmit: handleSelectionSubmit,
      onCancel: handleSelectionCancel,
    });
  };

  return (
    <div className="select-multiple">
      {isPresent(toggleText) && (
        <Toggle
          label={toggleText}
          active={toggleValue}
          onClick={() => {
            if (selected.size === 0 && toggleValue && options.length) {
              handleEdit();
            }
            onToggleChange(!toggleValue);
          }}
        />
      )}
      <Fold open={!toggleValue && selected.size > 0}>
        {isPresent(toggleText) && <SizedBox height={ThemeSpacing.Xl} />}
        <div className="selected">
          {selectedOptions.map((o) => (
            <Chip text={o.label} key={o.id} />
          ))}
          {selectedOptions.length > 5 && <Chip text={counterText(selected.size - 5)} />}
        </div>
        <SizedBox height={ThemeSpacing.Md} />
        <button type="button" onClick={handleEdit}>
          {editText}
        </button>
      </Fold>
      <FieldError error={error} />
    </div>
  );
};
