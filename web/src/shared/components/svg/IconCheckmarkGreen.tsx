import type { SVGProps } from 'react';
const SvgIconCheckmarkGreen = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    viewBox="0 0 22 22"
    {...props}
  >
    <defs>
      <clipPath id="icon-checkmark-green_svg__a">
        <path
          d="M0 0h22v22H0z"
          style={{
            opacity: 0,
            fill: '#14bc6e',
          }}
        />
      </clipPath>
      <style>{'.icon-checkmark-green_svg__c{fill:#14bc6e}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#icon-checkmark-green_svg__a)',
      }}
      transform="rotate(90 11 11)"
    >
      <rect
        width={12}
        height={2}
        className="icon-checkmark-green_svg__c"
        rx={1}
        transform="rotate(45 -.036 10.5)"
      />
      <rect
        width={8}
        height={2}
        className="icon-checkmark-green_svg__c"
        rx={1}
        transform="rotate(-45 23.616 -2.775)"
      />
    </g>
  </svg>
);
export default SvgIconCheckmarkGreen;
