// Puzzle Holocron: a Jedi holocron's puzzle as a blade (round 17 rework, Creyon: no small figures,
// a more fitting effect and model). The blade is the holocron's crystal core inside a lattice of
// interlocking panels: square panels tile it, their seams glowing, and one row after another the
// puzzle turns, its panels sliding round the blade and locking with a flash of light along their
// seams, while the core's light pulses through the gaps between them. When a turn locks, a band of
// light sweeps up the blade. A clash flares every seam.
// @part Turning panels
// @part Glowing seams
// @room 6
// @palette {"corona.rim_cool":[0.3,0.3,0.4],"corona.rim_hot":[0.85,0.85,0.95],"corona.inner":[0.95,0.95,1.0],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.8,0.8,0.95],"core.fringe_hot":[0.95,0.95,1.0],"light.color":[0.9,0.9,1.0],"trail":[0.85,0.85,1.0]}

const vec3 PH_LIGHT = vec3(0.6, 0.8, 1.0);
const float PH_ROW = 2.4;
// the puzzle's turn: which row turns (0 to 12), how far through (0 to 1), and the locking flash
float ph_k(){ return floor(uTime*0.4); }
float ph_q(){ return fract(uTime*0.4); }

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float l = lvl(g);
  float L = uLength;
  float row = floor(b.y/PH_ROW);
  float k = ph_k(), q = ph_q();
  float turning = 1.0 - step(0.5, abs(row - mod(k*5.0, 14.0)));
  float slide = smoothstep(0.0, 0.7, q)*turning*(mod(row, 2.0) < 0.5 ? 1.0 : -1.0);
  // panels across the blade: the turning row's slide along, as if round it
  float across = b.x*4.0 + slide*1.0;
  vec2 f = vec2(fract(across), fract(b.y/PH_ROW));
  float seamX = halo(0.5 - abs(f.x - 0.5), 0.06), seamY = halo(0.5 - abs(f.y - 0.5), 0.06/PH_ROW*2.4);
  float seam = max(seamX, seamY);
  float lock = exp(-pow((q - 0.72)/0.05, 2.0))*turning;
  float gap = halo(0.5 - abs(f.x - 0.5), 0.12)*0.5;
  // the panels darker than the core's light between them, each a slightly different shade
  float shade = 0.65 + 0.2*h11(floor(across) + row*7.0);
  vec3 c = PH_LIGHT*shade*(1.0 - 0.3*part(0.0)*(1.0 - seam)) + vec3(0.85, 0.95, 1.0)*(seam*(0.8 + 2.5*lock) + gap)*part(1.0);
  // the band of light sweeping up when a turn locks
  float band = halo(b.y - (q - 0.72)*L*3.0, 1.5)*step(0.72, q);
  return l*(c + vec3(1.0)*band*0.8 + vec3(1.0)*seam*clamp(uClash, 0.0, 1.0)*1.5);
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c*(0.9 + 0.1*sin(uTime*2.0 + b.y*0.2)); }

// the core's light through the lattice: a soft glow, brightening as each turn locks (flat, facing
// the camera, as Sol's are)
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength;
  float y0 = L*0.15, y1 = L*0.97;
  float q = ph_q();
  float lock = exp(-pow((q - 0.72)/0.05, 2.0));
  vec3 fx = fxAura(ro, rd, y0, y1, 1.8, PH_LIGHT)*(0.35 + 0.8*lock)*part(1.0);
  // round 14: a chroma for the blade only; its effects keep their own colours
  uKeep += fx*uFx;
  return vec3(0.0);
}
