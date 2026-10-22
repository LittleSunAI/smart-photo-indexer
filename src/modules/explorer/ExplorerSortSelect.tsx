import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {sortHelp} from '../../shared/help/SelectionHelp';
import {CompactSelect,type SelectOption} from '../../shared/ui/CompactSelect';
import {messages} from '../../shared/i18n';
import {searchOrders,normalizeSearchOrder} from '../search/SearchOrder';
import {sortLabel} from './ExplorerViews';
import type {SortMode} from '../../app/types';

export const explorerSortOptions:SelectOption[]=[
 {value:'dateTakenNewest',get label() { return localeText("main", "ui.ExplorerSortSelect.a46dcd8e648edcbb"); },get title() { return localeText("main", "ui.ExplorerSortSelect.c9268ae9d9aafdc3"); }},
 {value:'dateTakenOldest',get label() { return localeText("main", "ui.ExplorerSortSelect.f4199adf6df8a112"); },get title() { return localeText("main", "ui.ExplorerSortSelect.3c0bad33a8fe9590"); }},
 {value:'modifiedDesc',get label() { return localeText("main", "ui.ExplorerSortSelect.a97f824232e67652"); },get title() { return localeText("main", "ui.ExplorerSortSelect.4b5ae3b5a451a732"); }},
 {value:'modifiedAsc',get label() { return localeText("main", "ui.ExplorerSortSelect.1a9ac27f3eb401ce"); },get title() { return localeText("main", "ui.ExplorerSortSelect.b0ab9c6edb737343"); }},
 {value:'analyzedDesc',get label() { return localeText("main", "ui.ExplorerSortSelect.8ed54460bf72d6e4"); },get title() { return localeText("main", "ui.ExplorerSortSelect.68273123682ecedb"); }},
 {value:'analyzedAsc',get label() { return localeText("main", "ui.ExplorerSortSelect.d20b420f56d23d33"); },get title() { return localeText("main", "ui.ExplorerSortSelect.466ebe804169169f"); }},
 {value:'nameAsc',get label() { return localeText("main", "ui.ExplorerSortSelect.6b5b596629b533b1"); }}, {value:'nameDesc',get label() { return localeText("main", "ui.ExplorerSortSelect.aa3a74da40b468ee"); }},
 {value:'readyFirst',get label() { return localeText("main", "ui.ExplorerSortSelect.216a07b0057651af"); }}, {value:'readyLast',get label() { return localeText("main", "ui.ExplorerSortSelect.e29bfcc19b37c44e"); }},
 {value:'errorsFirst',get label() { return localeText("main", "ui.ExplorerSortSelect.f8e389bae38a6fdb"); }}, {value:'errorsLast',get label() { return localeText("main", "ui.ExplorerSortSelect.6be5169dc1a2d98b"); }},
 {value:'type',get label() { return localeText("main", "ui.ExplorerSortSelect.1608f0d518429787"); }},
];
export function ExplorerSortSelect({value,queryWorkspace,onChange}:{value:string;queryWorkspace:boolean;onChange(value:string):void}){
  useCurrentLocale();
 const selected=queryWorkspace?normalizeSearchOrder(value):value;
 const options=queryWorkspace?searchOrders().map(([key,label])=>({value:key,label,title:explorerSortOptions.find(o=>o.value===key||key==='takenDesc'&&o.value==='dateTakenNewest'||key==='takenAsc'&&o.value==='dateTakenOldest')?.title})):explorerSortOptions;
 const extra=!options.some(o=>o.value===selected)?[{value:selected,label:selected==='statusAsc'?localeText("main", "ui.ExplorerSortSelect.476966742118c274"):sortLabel(selected as SortMode)}]:[];
 return <CompactSelect tourId="main.display.sort" helpId="sort" caption={messages().toolbar.sortBy} className="compact-select--sort" value={selected} onChange={onChange} options={[...extra,...options]} message={{id:"sort.control"}} supplement={sortHelp(selected)}/>;
}
