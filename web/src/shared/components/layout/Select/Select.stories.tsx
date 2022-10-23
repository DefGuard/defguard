import { Story } from '@ladle/react';
import { useMemo, useState } from 'react';

import { Select, SelectOption } from './Select';

const storySelectOptions: SelectOption<number>[] = [
  {
    label: 'Option 1',
    value: 1,
    key: 1,
  },
  {
    label: 'Option 2',
    value: 2,
    key: 2,
  },
  {
    label: 'Option 3',
    value: 3,
    key: 3,
  },
];

export const SelectStory: Story<{
  loading?: boolean;
  disabled?: boolean;
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
}> = ({ loading = false, disabled = false }) => {
  const [selected, setSelected] = useState<SelectOption<number>[]>([]);
  const [searchTerm, setSearchTerm] = useState<string | undefined>();

  const getFilteredOptions = useMemo(() => {
    if (searchTerm && searchTerm.length > 0) {
      return storySelectOptions.filter((o) => {
        if (o.label.toLowerCase().includes(searchTerm)) {
          return true;
        }
        if (
          o.value.toString().toLowerCase().includes(searchTerm.toLowerCase()) ||
          o.value.toString() === searchTerm
        ) {
          return true;
        }
        return false;
      });
    }
    return storySelectOptions;
  }, [searchTerm]);

  return (
    <Select
      multi={true}
      selected={selected}
      onChange={(newState) => {
        if (Array.isArray(newState)) {
          setSelected(newState);
        } else {
          setSelected([]);
        }
      }}
      onSearch={(term) => setSearchTerm(term)}
      searchDebounce={0}
      searchable={true}
      options={getFilteredOptions}
      loading={loading}
      placeholder="Placeholder"
      disabled={disabled}
      outerLabel="Outer label"
    />
  );
};
