import { useMemo } from 'react';
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
  onChange,
  options,
  selected,
  error,
}: SelectMultipleProps<T, M>) => {
  const selectedOptions = useMemo(
    () => options.filter((o) => selected.has(o.id)),
    [options, selected],
  );

  const handleEdit = () => {
    useSelectionModal.setState({
      isOpen: true,
      title: modalTitle,
      options,
      //@ts-expect-error
      selected: selected,
      //@ts-expect-error
      onSubmit: onChange,
    });
  };

  return (
    <div className="select-multiple">
      {isPresent(toggleText) && (
        <Toggle
          label={toggleText}
          active={selected.size === 0}
          onClick={() => {
            if (selected.size === 0) {
              handleEdit();
            } else {
              onChange([]);
            }
          }}
        />
      )}
      <Fold open={selected.size > 0 || !isPresent(toggleText)}>
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
