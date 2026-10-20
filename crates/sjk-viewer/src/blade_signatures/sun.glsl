// Sun: Sol's own blade, blade 001, a star held in the hand. A white-gold core under an orange
// corona that breathes slowly, calm and huge where the Starforge is busy: dark sunspots drift
// along the core, long thin corona streamers reach out and fade, a solar wind of pale motes
// rides out to the tip, and the widest warm glow of any blade. It lights as a sunrise, deep red
// at first and warming to white gold as it grows, with a dawn flare at the emitter once it is
// out; it goes out as a sunset, cooling to deep red as it slides back in. A swing makes the
// corona swell and the wind blow harder; a clash tears a coronal mass ejection off the blade,
// a loop of plasma that arcs away and breaks off.
// @part Sunspots and corona
// @arcs Prominences
// @motes Sparks
// @star Glint
// @haze Haze
// @room 10
// @palette {"corona.rim_cool":[1.0,0.2,0.02],"corona.rim_hot":[1.0,0.42,0.05],"corona.inner":[1.0,0.72,0.24],"core.white":[1.0,0.97,0.86],"core.fringe_cool":[1.0,0.42,0.05],"core.fringe_hot":[1.0,0.72,0.24],"light.color":[1.5,0.85,0.3],"trail":[1.0,0.6,0.12]}


const vec3 SU_WHITE = vec3(1.0, 0.96, 0.84);
const vec3 SU_GOLD = vec3(1.0, 0.7, 0.26);
const vec3 SU_ORANGE = vec3(1.0, 0.42, 0.06);
const vec3 SU_RED = vec3(1.0, 0.16, 0.03);

// the slow breath of the whole star (about five seconds), 0 to 1
float su_breath(){ return 0.5 + 0.5*sin(uTime*1.25 + uSeed*6.2832); }
// how far the sunrise has warmed (0 deep red, 1 white gold) and the sunset cooled (1 deep red)
float su_dawn(){ return uAge >= 2000.0 ? 1.0 : smoothstep(0.0, 0.9, uAge); }
float su_dusk(){ return uAge >= 2000.0 ? smoothstep(0.0, 0.3, uAge - 2000.0) : 0.0; }
// a colour seen at sunrise or sunset: its level kept, its hue a red sun low on the horizon
vec3 su_sky(vec3 c){
  vec3 low = lvl(c)*mix(SU_RED, SU_ORANGE, 0.35);
  float red = max(1.0 - su_dawn(), su_dusk());
  return mix(c, low, red*0.85);
}
// sunspots at height y: dark umbrae drifting slowly along the blade, 0 to 1
float su_spot(float y){
  float n = vn(vec2(y*0.3 - uTime*0.11, uSeed*7.0 + 3.0));
  float n2 = vn(vec2(y*0.55 + uTime*0.07, uSeed*3.0 + 11.0));
  float spot = max(smoothstep(0.7, 0.86, n), smoothstep(0.76, 0.9, n2)*0.8)*part(0.0);
  return spot*smoothstep(1.5, 4.0, y)*(1.0 - smoothstep(uLength - 3.0, uLength, y));
}
// where a clash lands on the blade
float su_clash_y(){ return uLength*0.62; }

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float br = su_breath();
  float top = uLength*uIgn;
  float cl = clamp(uClash, 0.0, 1.0);
  // the corona breathes and swells with a swing; a clash lights the place it lands
  vec3 o = g*(0.92 + 0.16*br + 0.3*uSwing);
  // the sunspots darken the glow next to the core too, so they show from afar
  o = mix(o, o*vec3(1.0, 0.45, 0.15)*0.6, su_spot(b.y)*0.7*exp(-pow(b.x/max(r*0.8, px), 2.0)));
  o += g*1.4*cl*exp(-pow((b.y - su_clash_y())/4.0, 2.0));
  // the rising sun: a white-hot edge climbing the blade while it grows
  float rising = 1.0 - smoothstep(0.7, 1.0, uIgn);
  o += SU_WHITE*lvl(g)*1.6*exp(-pow((b.y - top)/2.2, 2.0))*rising;
  return su_sky(o);
}

vec3 sig_core(vec3 c, vec2 b, float px){
  // sunspots: the core runs deep orange where they drift past
  vec3 o = mix(c, c*vec3(1.0, 0.42, 0.12)*0.7, su_spot(b.y)*0.85);
  o *= 0.95 + 0.1*su_breath();
  return su_sky(o);
}

// a coronal mass ejection: a loop of plasma torn off the blade at height y on side a, a its
// age (0 to 1): it swells out, its top breaks away and drifts off as a fading blob
vec3 su_cme(vec3 ro, vec3 rd, float px, float y, float a, float side){
  float s, ta;
  vec3 col = vec3(0.0);
  float ex = 1.0 - exp(-3.5*a);
  float H = 0.6 + 8.5*ex;
  float dy = 3.0 + 3.5*ex;
  float best = 1e3, bs = 0.0;
  vec3 prev = around(0.4, side, y - dy*0.5);
  for (int j = 1; j <= 10; j++){
    float f = float(j)/10.0;
    float lift = H*sin(3.1416*f);
    vec3 q = around(0.4 + lift, side + 0.03*sin(3.1416*f*2.0 + a*4.0), y - dy*0.5 + dy*f);
    float d = raySeg(ro, rd, prev, q, s);
    if (d < best){ best = d; bs = (float(j) - 1.0 + s)/10.0; }
    prev = q;
  }
  // the loop's top tears first, so its middle fades as the ends hold on, then all of it goes
  float torn = smoothstep(0.35, 0.7, a)*exp(-pow((bs - 0.5)/0.18, 2.0));
  float life = (1.0 - smoothstep(0.55, 1.0, a))*(1.0 - torn);
  float fl = 0.6 + 0.6*vn(vec2(bs*10.0 - a*6.0, side*9.0));
  col += (mix(SU_WHITE, SU_GOLD, a)*halo(best, max(0.1 + 0.12*fl, px))*1.6*fl + SU_ORANGE*halo(best, 0.8)*0.5 + SU_RED*halo(best, 1.8)*0.15)*life;
  // the torn-off plasma flying on outward
  vec3 blob = around(0.4 + H*1.15 + 6.0*smoothstep(0.35, 1.0, a), side, y);
  float db = rayPt(ro, rd, blob, ta);
  float gone = smoothstep(0.35, 0.5, a)*(1.0 - smoothstep(0.7, 1.0, a));
  col += (SU_GOLD*halo(db, max(0.5, px))*1.3 + SU_ORANGE*halo(db, 1.6)*0.45)*gone;
  return col;
}

vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, br = su_breath();
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.12, y1 = L*0.96;
  vec3 col = vec3(0.0);
  float corona = part(0.0)*uFx;

  // the widest warm glow of any blade, breathing with the star
  col += fxAura(ro, rd, y0, y1, 4.6 + 0.6*br + 1.2*sw, SU_ORANGE)*(1.4 + 0.5*br)*corona;
  // corona streamers: long thin rays reaching out slowly and fading, longer in a swing
  col += fxRays(ro, rd, px, y0, y1, 7.0, 6.5 + 2.0*sw, 0.11, 3.7, SU_GOLD)*(0.55 + 0.25*br)*corona;
  // the solar wind: pale motes riding out along the blade to the tip
  col += fxMotes(ro, rd, px, y0, y1 + 1.0, 0.9, 4.2, 16.0, 0.07 + 0.05*sw, 1.9, mix(SU_GOLD, SU_WHITE, 0.4))*(0.8 + 0.6*sw)*corona;

  // the dawn: once the blade is out, a flare at the emitter and a warm bloom up the blade
  float dawn = smoothstep(0.25, 0.45, uAge)*(1.0 - smoothstep(0.6, 1.8, uAge))*step(uAge, 1000.0);
  if (dawn > 0.001){
    col += fxFlare(ro, rd, vec3(0.0, 1.2, 0.0), px, 0.45, 9.0, 4.0, 0.4, SU_GOLD)*dawn*1.4;
    col += fxAura(ro, rd, 0.0, L, 2.0 + 6.0*smoothstep(0.25, 1.4, uAge), mix(SU_WHITE, SU_GOLD, 0.5))*dawn*1.6;
  }

  // the clash: a white flash and a coronal mass ejection torn off the blade
  if (cl > 0.01){
    float a = 1.0 - cl;
    float yc = su_clash_y();
    float ta;
    float dc = rayPt(ro, rd, vec3(0.0, yc, 0.0), ta);
    col += SU_WHITE*(halo(dc, 1.2)*2.2 + halo(dc, 3.0)*0.6)*cl*cl;
    // to one side of the blade's picture (a loop towards the camera would hide its arch)
    float side = h11(uSeed*7.1) < 0.5 ? 0.25 : 0.75;
    col += su_cme(ro, rd, px, yc, a, side)*uFx;
  }
  return su_sky(col);
}
