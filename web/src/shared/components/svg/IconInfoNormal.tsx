import type { SVGProps } from 'react';
const SvgIconInfoNormal = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={18}
    height={18}
    viewBox="0 0 18 18"
    {...props}
  >
    <defs>
      <style>{'.icon-info-normal_svg__icon-info-normal-b{fill:#fff}'}</style>
    </defs>
    <g transform="translate(-786 -145)">
      <circle
        cx={9}
        cy={9}
        r={9}
        style={{
          fill: '#899ca8',
        }}
        transform="translate(786 145)"
      />
      <rect
        width={2}
        height={7}
        className="icon-info-normal_svg__icon-info-normal-b"
        rx={1}
        transform="translate(794 152)"
      />
      <rect
        width={2}
        height={2}
        className="icon-info-normal_svg__icon-info-normal-b"
        rx={1}
        transform="translate(794 149)"
      />
    </g>
  </svg>
);
export default SvgIconInfoNormal;
