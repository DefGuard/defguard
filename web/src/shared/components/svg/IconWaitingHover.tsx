import type { SVGProps } from 'react';
const SvgIconWaitingHover = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    viewBox="0 0 22 22"
    {...props}
  >
    <defs>
      <clipPath id="a">
        <path
          d="M0 0h22v22H0z"
          style={{
            opacity: 0,
            fill: '#0c8ce0',
          }}
        />
      </clipPath>
      <style>{'.c{fill:#0c8ce0}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#a)',
      }}
      transform="rotate(-90 11 11)"
    >
      <path
        d="m9.41 14.207 1.882-2.51h3.834a.7.7 0 0 0 0-1.394h-4.183a.7.7 0 0 0-.558.279l-2.091 2.789a.7.7 0 0 0 1.115.837Z"
        className="c"
      />
      <path
        d="M20 11a9 9 0 1 0-9 9 9.01 9.01 0 0 0 9-9ZM3.394 11A7.606 7.606 0 1 1 11 18.606 7.615 7.615 0 0 1 3.394 11Z"
        className="c"
      />
    </g>
  </svg>
);
export default SvgIconWaitingHover;
