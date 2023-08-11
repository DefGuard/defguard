import type { SVGProps } from 'react';
const SvgIconFilter = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    viewBox="0 0 22 22"
    {...props}
  >
    <defs>
      <clipPath id="icon-filter_svg__a">
        <path
          d="M0 0h22v22H0z"
          style={{
            opacity: 0,
            fill: '#899ca8',
          }}
          transform="rotate(90 -303 906)"
        />
      </clipPath>
    </defs>
    <g
      style={{
        clipPath: 'url(#icon-filter_svg__a)',
      }}
    >
      <path
        d="M16 3.794a.877.877 0 0 0-.877-.877H.877A.877.877 0 0 0 .16 4.3l5.165 7.344.015 5.343a1.7 1.7 0 0 0 2.639 1.408l1.845-1.23A1.753 1.753 0 0 0 10.6 15.7l-.019-4.021L15.838 4.3A.877.877 0 0 0 16 3.794Zm-7.17 7.323.021 4.583-1.758 1.175-.016-5.79L2.566 4.67h10.857Z"
        style={{
          fill: '#899ca8',
        }}
        transform="translate(3 .203)"
      />
    </g>
  </svg>
);
export default SvgIconFilter;
