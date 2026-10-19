// Dathomir Blood Moon: the Nightsisters' red night as a blade. A dark crimson blade with a shadowed
// heart and a slow red tide climbing it; drops of red light drip off it and fall, a red mist curls up
// off it, two dark red streams wind up beside it and shadow copies trail every swing. A clash flares it.
// @part Blood drips
// @part Red mist
// @part Moon streams
// @room 8
// @palette {"corona.rim_cool":[0.35,0.0,0.02],"corona.rim_hot":[1.0,0.12,0.08],"corona.inner":[0.25,0.0,0.02],"core.white":[0.55,0.05,0.05],"core.fringe_cool":[0.6,0.02,0.02],"core.fringe_hot":[1.0,0.2,0.15],"light.color":[0.9,0.08,0.06],"trail":[0.9,0.08,0.06]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float tide = smoothstep(0.3, 1.0, sin(b.y*0.2 - uTime*0.9));
  float heart = 1.0 - 0.45*exp(-pow(b.x/0.25, 2.0));
  return g*heart*(0.8 + 0.5*tide);
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 red = vec3(1.0, 0.08, 0.05), dark = vec3(0.55, 0.0, 0.02);
  fx += fxFall(ro, rd, px, y0, y1, 1.4, 12.0, 5.0, 0.6, 0.0, 2.9, red)*part(0.0);
  fx += fxMotes(ro, rd, px, y0, y1, 0.6, 1.8, 10.0, -0.05, 4.4, red)*0.8*part(0.0);
  fx += fxSmoke(ro, rd, y0, y1, 3.6, 2.0, 5.3, dark)*1.2*part(1.0);
  fx += fxStreams(ro, rd, px, y0, y1, 2.0, 3.2, 0.18, 1.2, 0.7, dark*1.4)*part(2.0);
  return fx*uFx;
}
