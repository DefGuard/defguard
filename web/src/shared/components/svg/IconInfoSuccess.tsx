import type { SVGProps } from 'react';
const SvgIconInfoSuccess = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={18}
    height={18}
    viewBox="0 0 18 18"
    {...props}
  >
    <defs>
      <style>{'.icon-info-success_svg__b{fill:#fff}'}</style>
    </defs>
    <g transform="translate(-786 -145)">
      <circle
        cx={9}
        cy={9}
        r={9}
        style={{
          fill: '#14bc6e',
        }}
        transform="translate(786 145)"
      />
      <g transform="translate(-.543)">
        <rect
          width={8}
          height={2}
          className="icon-info-success_svg__b"
          rx={1}
          transform="rotate(135 368.746 241.708)"
        />
        <rect
          width={5}
          height={2}
          className="icon-info-success_svg__b"
          rx={1}
          transform="rotate(45 212.04 1032.873)"
        />
      </g>
    </g>
  </svg>
);
export default SvgIconInfoSuccess;
