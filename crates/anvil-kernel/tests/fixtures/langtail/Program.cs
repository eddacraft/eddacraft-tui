// LANGTAIL T1 fixture — representative C# source (ASP.NET-flavoured).
using System;
using System.Collections.Generic;
using System.Threading.Tasks;

namespace Example.App
{
    public class Service
    {
        private readonly string _name;

        public Service(string name)
        {
            _name = name;
        }

        public Task<IReadOnlyList<string>> FetchAsync()
        {
            return Task.FromResult<IReadOnlyList<string>>(new List<string>());
        }

        private int Helper() => 0;
    }

    public interface IRepository
    {
        IReadOnlyList<string> FindAll();
    }

    public enum Status
    {
        Active,
        Inactive
    }
}
