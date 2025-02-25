import './style.scss';

import clsx from 'clsx';
import dayjs from 'dayjs';
import { forwardRef, HTMLAttributes, useState } from 'react';
import DatePicker, { ReactDatePickerCustomHeaderProps } from 'react-datepicker';

import { useAppStore } from '../../../hooks/store/useAppStore';

export const DateInput = () => {
  const locale = useAppStore((s) => s.language);
  const [selected, setSelected] = useState<Date | null>(new Date());
  return (
    <>
      <DatePicker
        selected={selected}
        onChange={(val) => {
          console.log(val);
          console.log(typeof val);
          setSelected(val);
        }}
        customInput={<DisplayField selected={selected} />}
        renderCustomHeader={CustomHeader}
        // eslint-disable-next-line @typescript-eslint/no-unused-vars
        renderDayContents={(day, _) => <CustomDay day={day} />}
        open={true}
        locale={locale}
        showTimeSelect={false}
        closeOnScroll
      />
    </>
  );
};

type DayProps = {
  day: number;
};

const CustomDay = ({ day }: DayProps) => {
  return (
    <div className="custom-day">
      <span>{day}</span>
    </div>
  );
};

const CustomHeader = ({
  decreaseMonth,
  increaseMonth,
  date,
}: ReactDatePickerCustomHeaderProps) => {
  const displayDate = () => {
    return dayjs(date).format('MMMM YYYY');
  };
  return (
    <div className="date-picker-custom-header">
      <button className="icon-container" type="button" onClick={decreaseMonth}>
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width={22}
          height={22}
          viewBox="0 0 22 22"
          fill="none"
        >
          <path
            d="M11.8777 6.05022L7.6351 10.2929C7.24458 10.6834 7.24458 11.3165 7.6351 11.7071C8.02563 12.0976 8.65879 12.0976 9.04932 11.7071L13.292 7.46443C13.6825 7.07391 13.6825 6.44074 13.292 6.05022C12.9014 5.65969 12.2683 5.65969 11.8777 6.05022Z"
            fill="#899CA8"
          />
          <path
            d="M7.63625 12.0502L11.8789 16.2929C12.2694 16.6834 12.9026 16.6834 13.2931 16.2929C13.6836 15.9023 13.6836 15.2692 13.2931 14.8786L9.05046 10.636C8.65994 10.2455 8.02677 10.2455 7.63625 10.636C7.24572 11.0265 7.24572 11.6597 7.63625 12.0502Z"
            fill="#899CA8"
          />
        </svg>
      </button>
      <p>{displayDate()}</p>
      <button className="icon-container" type="button" onClick={increaseMonth}>
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width={22}
          height={22}
          viewBox="0 0 22 22"
          fill="none"
        >
          <path
            d="M11.8777 6.05022L7.6351 10.2929C7.24458 10.6834 7.24458 11.3165 7.6351 11.7071C8.02563 12.0976 8.65879 12.0976 9.04932 11.7071L13.292 7.46443C13.6825 7.07391 13.6825 6.44074 13.292 6.05022C12.9014 5.65969 12.2683 5.65969 11.8777 6.05022Z"
            fill="#899CA8"
          />
          <path
            d="M7.63625 12.0502L11.8789 16.2929C12.2694 16.6834 12.9026 16.6834 13.2931 16.2929C13.6836 15.9023 13.6836 15.2692 13.2931 14.8786L9.05046 10.636C8.65994 10.2455 8.02677 10.2455 7.63625 10.636C7.24572 11.0265 7.24572 11.6597 7.63625 12.0502Z"
            fill="#899CA8"
          />
        </svg>
      </button>
    </div>
  );
};

type DisplayProps = {
  selected?: Date | null;
} & HTMLAttributes<HTMLButtonElement>;

const DisplayField = forwardRef<HTMLButtonElement, DisplayProps>(
  ({ selected, className, ...rest }, ref) => {
    return (
      <div className="date-input-container">
        <button
          {...rest}
          className={clsx('date-input', className)}
          ref={ref}
          type="button"
        >
          <span>{selected?.toISOString()}</span>
        </button>
      </div>
    );
  },
);
