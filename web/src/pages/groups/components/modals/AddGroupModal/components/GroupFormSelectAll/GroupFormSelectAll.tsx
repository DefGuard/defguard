import { Control, useWatch } from 'react-hook-form';

import { SelectRow } from '../../../../../../../shared/defguard-ui/components/Layout/SelectRow/SelectRow';
import { User } from '../../../../../../../shared/types';
import { ModifyGroupFormFields } from '../../AddGroupModal';

type Props = {
  // how many users are there
  control: Control<ModifyGroupFormFields>;
  users: User[];
};

export const GroupFormSelectAll = ({ users, control }: Props) => {
  const membersValue = useWatch({ control, name: 'members', defaultValue: [] });
  return (
    <SelectRow
      selected={(membersValue?.length ?? 0) === users.length}
      className="select-all"
    >
      <p>Select all users</p>
    </SelectRow>
  );
};
