#!/usr/bin/env python3
"""Transcribe static Rails sidebar markup, with ordinary typed dynamic bindings."""
import json,re
from pathlib import Path
root=Path(__file__).resolve().parents[2]
v=json.loads((root/'rust/crates/campfire/src/controllers/users/full_sidebar_vectors.json').read_text())
out=root/'rust/crates/views/templates/users/sidebars/composition';out.mkdir(parents=True,exist_ok=True)
def assets(html):
 return re.sub(r'/assets/([a-zA-Z0-9_./-]+)-[0-9a-f]{8,}([.][a-zA-Z0-9]+)',lambda m:'{{ ctx.asset("'+m[1]+m[2]+'") }}',html)
def save(name,html): (out/('_'+name+'.html')).write_text(assets(html)+'{{ "" }}')
empty=next(c for c in v['cases'] if c['name']=='empty');s=empty['html'];s=s[s.index('target="_top">')+len('target="_top">'):s.rindex('</turbo-frame>')]
s=s.replace('badge-dot dm-presence"','badge-dot dm-presence{% if sidebar.configured %} huddle-presence{% endif %}"')
s=s.replace('<span>3</span>','{% if let Some(path) = sidebar.logo_path.as_ref() %}{{ h::image_tag(ctx,path.as_str(),h::attrs().alt("").size(36)) }}{% else %}<span>{{ self.account_initial() }}</span>{% endif %}')
s=s.replace('>37signals<','>{{ sidebar.account_name }}<')
s=s.replace('>Kevin<','>{{ sidebar.actor.name }}<').replace('712064548','{{ sidebar.actor.id }}')
s=s.replace(empty['input']['actor']['avatar_path'],'{{ sidebar.actor.avatar_path }}')
# Preserve the actual ERB collection indentation and surrounding whitespace.
for element,field,indent in [('shared_rooms','channels','              '),('board_rooms','boards','              '),('voice_rooms','voice','            '),('stage_rooms','stage','              ')]:
 old=f'<div id="{element}" class="sidebar-list" data-controller="sorted-list">\n        </div>'
 assert old in s,element
 s=s.replace(old,f'<div id="{element}" class="sidebar-list" data-controller="sorted-list">\n{{{{ self.rows(&sidebar.{field}, "{indent}")|safe }}}}        </div>')
s=s.replace('            \n          </div>', '{{ self.rows(&sidebar.direct, "")|safe }}          </div>',1)
s=s.replace('            \n          </div>', '            {{ self.placeholders()|safe }}\n          </div>',1)
# Collection render has one indentation before the whole string, not per row.
s=s.replace('{{ self.rows(&sidebar.direct, "")|safe }}','            {{ self.direct_rows()|safe }}\n')
s=s.replace('</nav>\n\n\n','</nav>\n\n{% if sidebar.favorites.is_empty() %}\n{% endif %}')
s=s.replace('      <section class="sidebar-section sidebar-section--channels', '{{ self.favorites()|safe }}      <section class="sidebar-section sidebar-section--channels',1)
s=s.replace('      \n<details class="room-category__new">','      {{ self.categories()|safe }}\n<details class="room-category__new">')
s=s.replace('<input type="hidden" name="authenticity_token" value="post:/room_categories" />','{{ h::token_tag("/room_categories","post") }}')
# The five conditional create links are copied from the real open-policy render.
open_case=next(c for c in v['cases'] if c['name']=='false_false_127326141_false')
for heading,kind in [('channels','open'),('boards','board'),('voice','voice'),('stage','stage')]:
 pattern=rf'(<h2 id="{heading}-heading">[^<]+</h2>\n)(.*?)(        </header>)'
 match=re.search(pattern,open_case['html'],re.S);assert match
 button=match[2];s=s.replace(f'<h2 id="{heading}-heading">'+{'channels':'Channels','boards':'Boards','voice':'Voice','stage':'Stage'}[heading]+'</h2>\n',match[1]+'{% if sidebar.can_create %}'+button+'{% endif %}')
save('shell',s)
# Favorites are optional and every row keeps its own Rails partial.
fav=next(c for c in v['cases'] if c['name']=='false_false_127326141_true')['html']
start=fav.index('        <section class="sidebar-section sidebar-section--favorites')
end=fav.index('      <section class="sidebar-section sidebar-section--channels',start)
fav=fav[start:end]
fav=re.sub(r'(<div id="favorite_rooms" class="sidebar-list">\n).*?(          </div>)',r'\1{{ self.rows()|safe }}\2',fav,flags=re.S)
save('favorites',fav)
# The room menu is static; its asset bindings live in the shell above.
# Category controls bind id, name, collapsed state, tokens and member rows.
cat=next(p for p in v['parts'] if p['partial']=='category' and not p['category']['collapsed'])
s=cat['html'].split('<details class="room-category__new">')[0]
s=s[:s.rindex('</section>')+len('</section>')]+'\n'
s=s.replace('9301','{{ category.id }}').replace('Project &lt;&amp;&gt; team','{{ category.name }}')
s=s.replace('Collapse {{ category.name }}','{{ category.toggle_label() }} {{ category.name }}').replace('title="Collapse"','title="{{ category.toggle_label() }}"')
s=s.replace('src="/assets/disclosure-', 'src="/assets/disclosure-')
s=s.replace('<img aria-hidden="true" src="/assets/disclosure-26d63471.svg" width="16" height="16" />','{{ h::image_tag(ctx,"disclosure.svg",h::attrs().aria_hidden().size(16).attr_opt("class", category.collapsed.then_some("room-category__disclosure--collapsed"))) }}')
s=s.replace('name="room_category[collapsed]" value="true"','name="room_category[collapsed]" value="{{ !category.collapsed }}"')
s=re.sub(r'<input type="hidden" name="authenticity_token" value="(patch|delete):/room_categories/\{\{ category.id \}\}" />',lambda m:'{{ h::token_tag(category.action().as_str(),"'+m[1]+'") }}',s)
s=re.sub(r'(<div id="category_rooms_\{\{ category.id \}\}" class="sidebar-list)"(>\n).*?(    </div>)',r'\1{% if category.rows.is_empty() %} sidebar-list--empty{% endif %}"{% if category.collapsed %} hidden{% endif %}\2{{ self.rows()|safe }}\3',s,flags=re.S)
save('category',s)
