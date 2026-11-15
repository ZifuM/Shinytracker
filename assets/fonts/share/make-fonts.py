# Inter for the stats share images: subset to the characters used, digits made
# tabular for the number face, and Inter's GPOS kerning written into a classic
# 'kern' table (the app's font engine reads only that).
import sys
from fontTools.ttLib import TTFont, newTable
from fontTools.ttLib.tables._k_e_r_n import KernTable_format_0
from fontTools import subset

SRC = "/usr/share/fonts/opentype/inter/Inter-%s.otf"
CHARS = [c for c in range(0x20, 0x7F)] + [c for c in range(0xA0, 0x100)] + [0x2013, 0x2014, 0x2018, 0x2019, 0x201C, 0x201D, 0x2026, 0x2022, 0x00B7, 0x00D7, 0x2192, 0x2190, 0x2191, 0x2193, 0x2212, 0x2640, 0x2642, 0x2605]

def kern_value(font, lookups, g1, g2):
    total = 0
    for li in lookups:
        lk = font['GPOS'].table.LookupList.Lookup[li]
        for st in lk.SubTable:
            if lk.LookupType == 9:
                st = st.ExtSubTable
            if st.LookupType != 2 if hasattr(st,'LookupType') else False:
                continue
            cov = st.Coverage.glyphs
            if g1 not in cov:
                continue
            if st.Format == 1:
                ps = st.PairSet[cov.index(g1)]
                hit = None
                for r in ps.PairValueRecord:
                    if r.SecondGlyph == g2:
                        hit = r; break
                if hit is not None:
                    v = getattr(hit, 'Value1', None)
                    total += getattr(v, 'XAdvance', 0) or 0 if v else 0
                    break
            elif st.Format == 2:
                c1 = st.ClassDef1.classDefs.get(g1, 0)
                c2 = st.ClassDef2.classDefs.get(g2, 0)
                rec = st.Class1Record[c1].Class2Record[c2]
                v = getattr(rec, 'Value1', None)
                x = (getattr(v, 'XAdvance', 0) or 0) if v else 0
                # a class pair subtable applies (and stops) once g1 is covered
                total += x
                break
    return total

def kern_lookups(font):
    gpos = font['GPOS'].table
    idx = set()
    for fr in gpos.FeatureList.FeatureRecord:
        if fr.FeatureTag == 'kern':
            idx.update(fr.Feature.LookupListIndex)
    return sorted(idx)

def tnum_map(font):
    m = {}
    gsub = font['GSUB'].table
    for fr in gsub.FeatureList.FeatureRecord:
        if fr.FeatureTag == 'tnum':
            for li in fr.Feature.LookupListIndex:
                lk = gsub.LookupList.Lookup[li]
                for st in lk.SubTable:
                    if lk.LookupType == 7: st = st.ExtSubTable
                    if hasattr(st, 'mapping'): m.update(st.mapping)
    return m

def build(weight, out, tnum=False):
    f = TTFont(SRC % weight)
    cmap = f.getBestCmap()
    chars = [c for c in CHARS if c in cmap]
    tm = tnum_map(f) if tnum else {}
    use = {c: (tm.get(cmap[c], cmap[c])) for c in chars}
    lookups = kern_lookups(f)
    glyphs = sorted(set(use.values()))
    pairs = {}
    for a in glyphs:
        for b in glyphs:
            v = kern_value(f, lookups, a, b)
            if v:
                pairs[(a, b)] = v
    # subset to the glyphs we use (keeping their names)
    opts = subset.Options(); opts.layout_features = []; opts.name_IDs = ['*']; opts.notdef_outline = True; opts.glyph_names = True
    opts.drop_tables += ['GPOS', 'GSUB', 'GDEF']
    s = subset.Subsetter(opts); s.populate(glyphs=glyphs + ['.notdef'])
    s.subset(f)
    # cmap: each character to the glyph Chrome shows for it
    for t in f['cmap'].tables:
        if t.isUnicode():
            t.cmap = {c: use[c] for c in chars}
    k = newTable('kern'); k.version = 0; k.kernTables = []
    items = sorted(pairs.items())
    for i in range(0, len(items), 10000):
        st = KernTable_format_0(); st.version = 0; st.coverage = 1; st.format = 0; st.tupleIndex = None
        st.kernTable = dict(items[i:i+10000]); k.kernTables.append(st)
    f['kern'] = k
    f.save(out)
    print(out, len(chars), 'chars', len(pairs), 'kern pairs', 'missing', [hex(c) for c in CHARS if c not in cmap])

import os
o = sys.argv[1]; os.makedirs(o, exist_ok=True)
for w in ['Regular', 'SemiBold', 'Bold', 'ExtraBold', 'Black']:
    build(w, f"{o}/Inter-{w}.otf")
build('Black', f"{o}/Inter-Black-Tnum.otf", tnum=True)
