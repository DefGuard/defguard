import type { SVGProps } from 'react';
const SvgIconActivityAdd = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={16}
    height={16}
    viewBox="0 0 16 16"
    {...props}
  >
    <defs>
      <clipPath id="icon-activity-add_svg__a">
        <path
          d="M0 0h16v16H0z"
          style={{
            fill: '#cb3f3f',
          }}
        />
      </clipPath>
      <style>{'.icon-activity-add_svg__c{fill:#0c8ce0}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#icon-activity-add_svg__a)',
      }}
    >
      <rect
        width={10}
        height={2}
        className="icon-activity-add_svg__c"
        rx={1}
        transform="rotate(-90 10 3)"
      />
      <rect
        width={10}
        height={2}
        className="icon-activity-add_svg__c"
        rx={1}
        transform="rotate(-180 6.5 4.5)"
      />
    </g>
  </svg>
);
export default SvgIconActivityAdd;
