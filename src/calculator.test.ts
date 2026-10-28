import { describe, expect, it } from "vitest";
import {
  barMs,
  beatMs,
  decayRows,
  delayRows,
  double,
  formatBpm,
  formatMs,
  halve,
  isThreeTwo,
  threeHalves,
  twoThirds,
  isValidBpm,
  msToHz,
  noteMs,
  preDelayRows,
} from "./calculator";

const close = (a: number, b: number, digits = 2) => expect(a).toBeCloseTo(b, digits);

describe("delay maths at 120 BPM", () => {
  it("matches the textbook values", () => {
    expect(beatMs(120)).toBe(500);
    expect(barMs(120)).toBe(2000);
    const rows = Object.fromEntries(delayRows(120).map((r) => [r.id, r]));
    expect(rows["1/4"].straight).toBe(500);
    expect(rows["1/8"].straight).toBe(250);
    expect(rows["1/16"].straight).toBe(125);
    expect(rows["1/32"].straight).toBe(62.5);
    expect(rows["1/2"].straight).toBe(1000);
    expect(rows["1/1"].straight).toBe(2000);
    expect(rows["1/4"].dotted).toBe(750);
    expect(rows["1/8"].dotted).toBe(375);
    close(rows["1/4"].triplet, 333.333);
    close(rows["1/8"].triplet, 166.667);
    close(rows["1/16"].triplet, 83.333);
  });
});

describe("delay maths at awkward tempos", () => {
  it("handles 112 BPM (amapiano)", () => {
    close(beatMs(112), 535.7143, 3);
    close(noteMs(112, 0.5, "dotted"), 401.7857, 3);
    expect(formatMs(noteMs(112, 0.25))).toBe("133.93");
  });
  it("handles 97.5 BPM", () => {
    close(beatMs(97.5), 615.3846, 3);
  });
  it("handles 140 BPM (trap)", () => {
    close(beatMs(140), 428.5714, 3);
    close(noteMs(140, 0.5, "triplet"), 142.8571, 3);
  });
});

describe("reverb helpers", () => {
  it("pre-delay follows note lengths", () => {
    const rows = Object.fromEntries(preDelayRows(120).map((r) => [r.label, r.ms]));
    expect(rows["1/64"]).toBe(31.25);
    expect(rows["1/32"]).toBe(62.5);
    expect(rows["1/16"]).toBe(125);
    close(rows["1/16 triplet"], 83.333);
  });
  it("decay is expressed in beats and bars", () => {
    const rows = Object.fromEntries(decayRows(120).map((r) => [r.label, r.ms]));
    expect(rows["1/4"]).toBe(500);
    expect(rows["1 bar"]).toBe(2000);
    expect(rows["2 bars"]).toBe(4000);
  });
});

describe("helpers", () => {
  it("converts ms to Hz", () => {
    expect(msToHz(500)).toBe(2);
  });
  it("halves and doubles with two decimals", () => {
    expect(halve(149.34)).toBe(74.67);
    expect(double(97.5)).toBe(195);
  });
  it("validates BPM", () => {
    expect(isValidBpm(120)).toBe(true);
    expect(isValidBpm(0)).toBe(false);
    expect(isValidBpm(NaN)).toBe(false);
    expect(isValidBpm(1000)).toBe(false);
  });
  it("formats BPM without trailing zeros", () => {
    expect(formatBpm(112)).toBe("112");
    expect(formatBpm(97.5)).toBe("97.5");
    expect(formatBpm(103.72)).toBe("103.72");
  });

  it("fixes 3:2 mistakes such as 135 vs 90", () => {
    expect(twoThirds(135)).toBe(90);
    expect(threeHalves(90)).toBe(135);
    expect(isThreeTwo(135, 90)).toBe(true);
    expect(isThreeTwo(144, 96)).toBe(true);
    expect(isThreeTwo(140, 70)).toBe(false);
  });
});
