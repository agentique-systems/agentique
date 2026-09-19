"""Independent reference-XMI inspection; no Agentique transform/query helpers."""
from pathlib import Path
import xml.etree.ElementTree as ET

XID = '{http://www.omg.org/XMI}id'
XSI = '{http://www.w3.org/2001/XMLSchema-instance}type'


class Model:
    def __init__(self, directory, overrides=()):
        self.files = {}
        self.parents = {}
        self.ids = {}
        paths = {p.name:p for p in Path(directory).rglob('*.kermlx')}
        paths.update({p.name:p for p in overrides})
        for path in paths.values():
            root = ET.parse(path).getroot()
            for e in root.iter():
                self.files[e] = path.name
                if e.get(XID):
                    self.ids[(path.name, e.get(XID))] = e
                for child in e:
                    self.parents[child] = e

    def kind(self, e):
        return e.get(XSI, '').removeprefix('sysml:')

    def targets(self, e, prop):
        if e.get(prop):
            return [self.ids[(self.files[e], key)] for key in e.get(prop).split()]
        result = []
        for child in e:
            if child.tag == prop and child.get('href'):
                filename, key = child.get('href').split('#')
                result.append(self.ids[(Path(filename).name, key)])
        return result

    def name(self, e, visited=None):
        if e.get('declaredName'):
            return e.get('declaredName')
        visited = set() if visited is None else visited
        if e in visited:
            return ''
        visited.add(e)
        for rel in e:
            if self.kind(rel) == 'Redefinition':
                targets = self.targets(rel, 'redefinedFeature')
                return self.name(targets[0], visited) if targets else ''
        return ''

    def path(self, e):
        names = []
        while e is not None:
            if e.tag == 'ownedRelatedElement' or e.get('declaredName'):
                name = self.name(e)
                if name:
                    names.append(name)
            e = self.parents.get(e)
        return '::'.join(reversed(names))

    def find(self, path):
        candidates = [e for e in self.files if e.tag == 'ownedRelatedElement' and self.name(e) and self.path(e) == path]
        assert len(candidates) == 1, (path, len(candidates))
        return candidates[0]

    def record(self, e):
        return dict(file=self.files[e], id=e.get(XID), kind=self.kind(e), path=self.path(e),
                    xml=ET.tostring(e, encoding='unicode'))
