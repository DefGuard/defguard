import { useMemo } from 'react';
import './style.scss';
import { Chip } from '../../defguard-ui/components/Chip/Chip';
import { Fold } from '../../defguard-ui/components/Fold/Fold';
import { SizedBox } from '../../defguard-ui/components/SizedBox/SizedBox';
import { Toggle } from '../../defguard-ui/components/Toggle/Toggle';
import { ThemeSpacing } from '../../defguard-ui/types';
import { useSelectionModal } from '../modals/SelectionModal/useSelectionModal';
import type { SelectMultipleProps } from './types';

export const SelectMultiple = <T extends number | string>({
  counterText,
  editText,
  modalTitle,
  toggleText,
  onChange,
  options,
  selected,
}: SelectMultipleProps<T>) => {
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
      <Fold open={selected.size > 0}>
        <SizedBox height={ThemeSpacing.Xl} />
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
    </div>
  );
};
