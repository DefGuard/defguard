import './style.scss';
import dayjs from 'dayjs';
import { useState } from 'react';
import { Fragment } from 'react/jsx-runtime';
import { m } from '../../../../paraglide/messages';
import { Card } from '../../../../shared/components/Card/Card';
import { PolicyOsCard } from '../../../../shared/components/policyPostures/PolicyOsCard/PolicyOsCard';
import { SystemSelector } from '../../../../shared/components/SystemSelector/SystemSelector';
import { PolicyOsVariant } from '../../../../shared/components/SystemSelector/types';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Checkbox } from '../../../../shared/defguard-ui/components/Checkbox/Checkbox';
import { DateInput } from '../../../../shared/defguard-ui/components/DateInput/DateInput';
import type { DateRange } from '../../../../shared/defguard-ui/components/DateInput/types';
import { Helper } from '../../../../shared/defguard-ui/components/Helper/Helper';
import { Icon } from '../../../../shared/defguard-ui/components/Icon';
import { InteractiveBlock } from '../../../../shared/defguard-ui/components/InteractiveBlock/InteractiveBlock';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../../shared/defguard-ui/providers/snackbar/snackbar';
import { TooltipContent } from '../../../../shared/defguard-ui/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../../../shared/defguard-ui/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../../../shared/defguard-ui/providers/tooltip/TooltipTrigger';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { PlaygroundPolicyInfoListTest } from './components/PlaygroundPolicyInfoListTest/PlaygroundPolicyInfoListTest';
import { PlaygroundPolicyInfoListTestItem } from './components/PlaygroundPolicyInfoListTest/PlaygroundPolicyInfoListTestItem';
import { PlaygroundTestDrawer } from './components/PlaygroundTestDrawer';

const rangeFormat = 'DD/MM/YYYY HH:mm';

const formatRange = (range: DateRange): string =>
  `${dayjs(range.start).format(rangeFormat)} - ${dayjs(range.end).format(rangeFormat)}`;

export const PlaygroundNew = () => {
  const [dateRange, setDateRange] = useState<DateRange | null>(null);

  return (
    <div id="tab-new" className="tab">
      <PlaygroundTestDrawer />
      <SizedBox height={ThemeSpacing.Xl3} />
      <Card id="system-selector-test">
        {Object.values(PolicyOsVariant).map((variant) => (
          <SystemSelector
            key={variant}
            os={variant}
            onClick={() => {
              Snackbar.default(`${variant} clicked`);
            }}
          />
        ))}
      </Card>
      <SizedBox height={ThemeSpacing.Xl3} />
      <Card id="os-policy-card-test">
        {Object.values(PolicyOsVariant).map((variant) => (
          <Fragment key={variant}>
            <PolicyOsCard os={variant}>
              <p>{m.test_placeholder_long()}</p>
            </PolicyOsCard>
            <PolicyOsCard os={variant} hideCard>
              <p>{m.test_placeholder_long()}</p>
            </PolicyOsCard>
          </Fragment>
        ))}
      </Card>
      <SizedBox height={ThemeSpacing.Xl3} />
      <Card>
        <TooltipProvider>
          <TooltipTrigger>
            <Button variant="primary" text="Test light tooltip" />
          </TooltipTrigger>
          <TooltipContent variant="light">
            <p>{m.test_placeholder_long()}</p>
          </TooltipContent>
        </TooltipProvider>
      </Card>
      <SizedBox height={ThemeSpacing.Xl3} />
      {/* Example only implement the grid and styles accordingly for the screen design */}
      <Card>
        <PlaygroundPolicyInfoListTest>
          <PlaygroundPolicyInfoListTestItem icon="windows" label="Windows">
            <Icon icon="status-check" />
            <p>Windows 10 and higher</p>
          </PlaygroundPolicyInfoListTestItem>
          <PlaygroundPolicyInfoListTestItem>
            <Icon icon="status-check" />
            <p>Minimum 1 month update</p>
          </PlaygroundPolicyInfoListTestItem>
          <PlaygroundPolicyInfoListTestItem>
            <Icon icon="status-check" />
            <p>Disk encryption enabled</p>
          </PlaygroundPolicyInfoListTestItem>
        </PlaygroundPolicyInfoListTest>
      </Card>
      <SizedBox height={ThemeSpacing.Xl3} />
      <Card>
        <p>Helper block in checkbox</p>
        <SizedBox height={ThemeSpacing.Xl} />
        <InteractiveBlock
          variant="checkbox"
          value={false}
          title="Posture check name_1"
          content={m.test_placeholder()}
          helperBlock={
            <Helper>
              <p>{m.test_placeholder_extreme()}</p>
            </Helper>
          }
        />
        <SizedBox height={ThemeSpacing.Xl2} />
        <Checkbox
          active={false}
          text="Posture check name_2"
          helperBlock={
            <Helper>
              <p>{m.test_placeholder_extreme()}</p>
            </Helper>
          }
        />
      </Card>
      <SizedBox height={ThemeSpacing.Xl3} />
      <Card id="date-input-test">
        <p>Date input</p>
        <SizedBox height={ThemeSpacing.Xl} />
        <DateInput
          placeholder="Select date range"
          labels={{
            start: 'Start',
            end: 'End',
            reset: 'Reset',
            cancel: 'Cancel',
            apply: 'Apply',
          }}
          value={dateRange}
          onChange={setDateRange}
        />
        <SizedBox height={ThemeSpacing.Xl} />
        <p>{isPresent(dateRange) ? formatRange(dateRange) : 'No range selected'}</p>
      </Card>
      <SizedBox height={ThemeSpacing.Xl3} />
    </div>
  );
};
